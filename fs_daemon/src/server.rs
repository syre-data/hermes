use crate::event;
use notify_debouncer_full::{DebounceEventResult, DebouncedEvent, Debouncer, FileIdMap};
use std::{assert_matches::assert_matches, path::PathBuf};

const DEBOUNCE_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(100);

pub type EventSender = tokio::sync::mpsc::UnboundedSender<Vec<event::Event>>;
pub type EventReceiver = tokio::sync::mpsc::UnboundedReceiver<Vec<event::Event>>;
pub type CommandSender = crossbeam::channel::Sender<Command>;
pub type CommandReceiver = crossbeam::channel::Receiver<Command>;
type FsEventReceiver = crossbeam::channel::Receiver<DebounceEventResult>;

pub fn event_channel() -> (EventSender, EventReceiver) {
    tokio::sync::mpsc::unbounded_channel()
}

pub fn command_channel() -> (CommandSender, CommandReceiver) {
    crossbeam::channel::unbounded()
}

#[derive(Debug)]
pub enum Command {
    Watch(PathBuf),
    Unwatch(PathBuf),
}

type FileSystemWatcher = notify::RecommendedWatcher;
pub struct Daemon {
    fs_watcher: Debouncer<FileSystemWatcher, FileIdMap>,
    fs_event_rx: FsEventReceiver,
    command_rx: CommandReceiver,
    event_tx: EventSender,
}

impl Daemon {
    /// Create a new daemon to watch the file system and report events.
    /// Begins watching upon creation.
    pub fn new(event_tx: EventSender, command_rx: CommandReceiver) -> Self {
        let (fs_event_tx, fs_event_rx) = crossbeam::channel::unbounded();

        let fs_watcher =
            notify_debouncer_full::new_debouncer(DEBOUNCE_TIMEOUT, None, fs_event_tx).unwrap();

        Self {
            fs_watcher,
            fs_event_rx,
            event_tx,
            command_rx,
        }
    }

    /// Begin responding to events.
    pub fn run(&mut self) {
        self.listen_for_events();
    }

    /// Listen for events coming from child actors.
    fn listen_for_events(&mut self) {
        loop {
            crossbeam::select! {
                recv(self.command_rx) -> cmd => match cmd {
                    Ok(cmd) => self.handle_command(cmd),
                    Err(err) => panic!("{err:?}"),
                },
                recv(self.fs_event_rx) -> events => match events {
                    Ok(events) => self.handle_file_system_events(events),
                    Err(err) => panic!("{err:?}"),
                },
            }
        }
    }
}

impl Daemon {
    #[cfg_attr(feature = "tracing", tracing::instrument(level = "trace", skip(self)))]
    fn handle_command(&mut self, cmd: Command) {
        #[cfg(feature = "tracing")]
        tracing::trace!(?cmd);

        match cmd {
            Command::Watch(path) => self.watch_path(path),
            Command::Unwatch(path) => self.unwatch_path(path),
        }
    }

    /// Add a path to watch for file system changes.
    fn watch_path(&mut self, path: impl Into<PathBuf>) {
        let path: PathBuf = path.into();
        assert!(path.is_absolute());
        self.fs_watcher
            .watch(path, notify::RecursiveMode::Recursive)
            .unwrap();
    }

    /// Remove a path from watching file system changes.
    fn unwatch_path(&mut self, path: impl Into<PathBuf>) {
        let path: PathBuf = path.into();
        assert!(path.is_absolute());
        self.fs_watcher.unwatch(path).unwrap();
    }
}

impl Daemon {
    #[cfg_attr(feature = "tracing", tracing::instrument(level = "trace", skip(self)))]
    fn handle_file_system_events(&self, events: DebounceEventResult) {
        #[cfg(feature = "tracing")]
        tracing::trace!(?events);

        match events {
            Ok(events) => {
                let events = Self::filter_fs_events(events);
                #[cfg(feature = "tracing")]
                tracing::trace!("filtered events\n{events:?}");

                self.process_events(events)
            }
            Err(err) => {
                todo!("{err:?}")
            }
        }
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(level = "trace"))]
    fn filter_fs_events(mut events: Vec<DebouncedEvent>) -> Vec<DebouncedEvent> {
        use notify::{
            EventKind,
            event::{ModifyKind, RenameMode},
        };

        let relevant_events = events.iter().filter(|event| {
            matches!(
                event.kind,
                EventKind::Create(_)
                    | EventKind::Remove(_)
                    | EventKind::Modify(
                        ModifyKind::Any
                            | ModifyKind::Data(_)
                            | ModifyKind::Name(_)
                            | ModifyKind::Other
                    )
            )
        });

        let mut path_events = std::collections::HashMap::new();
        for event in relevant_events {
            match event.kind {
                EventKind::Create(_)
                | EventKind::Remove(_)
                | EventKind::Modify(ModifyKind::Any | ModifyKind::Data(_) | ModifyKind::Other) => {
                    let [path] = &event.paths[..] else {
                        panic!("invalid paths");
                    };

                    let entry = path_events.entry(path.clone()).or_insert(vec![]);
                    entry.push(event);
                }

                EventKind::Modify(ModifyKind::Name(RenameMode::Both)) => {
                    let [from, to] = &event.paths[..] else {
                        panic!("invalid paths");
                    };

                    let entry_from = path_events.entry(from.clone()).or_insert(vec![]);
                    entry_from.push(event);

                    let entry_to = path_events.entry(to.clone()).or_insert(vec![]);
                    entry_to.push(event);
                }

                EventKind::Modify(ModifyKind::Name(RenameMode::From))
                | EventKind::Modify(ModifyKind::Name(RenameMode::To)) => {
                    let [path] = &event.paths[..] else {
                        panic!("invalid paths");
                    };

                    let entry = path_events.entry(path.clone()).or_insert(vec![]);
                    entry.push(event);
                }

                EventKind::Modify(ModifyKind::Name(RenameMode::Any))
                | EventKind::Modify(ModifyKind::Name(RenameMode::Other)) => todo!(),

                EventKind::Access(_)
                | EventKind::Any
                | EventKind::Other
                | EventKind::Modify(ModifyKind::Metadata(_)) => {
                    unreachable!("filtered out beforehand")
                }
            }
        }
        #[cfg(feature = "tracing")]
        tracing::trace!("path grouped events\n{path_events:?}");

        for (_, path_events) in path_events.iter_mut() {
            let last = path_events
                .last()
                .expect("path should have at least one event");

            if matches!(last.kind, EventKind::Create(_) | EventKind::Remove(_)) {
                *path_events = vec![last];
                continue;
            }

            if path_events.len() == 3 {
                if matches!(
                    path_events[0].kind,
                    EventKind::Remove(
                        notify::event::RemoveKind::Any | notify::event::RemoveKind::File
                    )
                ) && matches!(
                    path_events[1].kind,
                    EventKind::Create(
                        notify::event::CreateKind::Any | notify::event::CreateKind::File
                    )
                ) && matches!(
                    path_events[2].kind,
                    EventKind::Modify(notify::event::ModifyKind::Any)
                ) {
                    // typically caused from app replacing file due to modifications.
                    *path_events = vec![path_events[2]];
                    continue;
                }
            }
        }
        #[cfg(feature = "tracing")]
        tracing::trace!("filtered path grouped events\n{path_events:?}");

        let relevant_events = path_events
            .into_iter()
            .flat_map(|(_, events)| events)
            .map(|relevant| {
                events
                    .iter()
                    .position(|event| relevant == event)
                    .expect("event to exist")
            })
            .collect::<Vec<_>>();

        let mut remove_idx = (0..events.len())
            .filter(|idx| !relevant_events.contains(idx))
            .collect::<Vec<_>>();
        remove_idx.sort();
        for idx in remove_idx.into_iter().rev() {
            events.remove(idx);
        }
        events
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(level = "trace", skip(self)))]
    fn process_events(&self, events: Vec<DebouncedEvent>) {
        let events = events
            .into_iter()
            .flat_map(|event| self.process_event(event))
            .collect::<Vec<_>>();
        #[cfg(feature = "tracing")]
        tracing::trace!(?events);

        self.event_tx.send(events).unwrap();
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(level = "trace", skip(self)))]
    fn process_event(&self, event: DebouncedEvent) -> Vec<event::Event> {
        match &event.kind {
            notify::EventKind::Create(_) => Self::process_event_create(event),
            notify::EventKind::Modify(_) => Self::process_event_modify(event),
            notify::EventKind::Remove(_) => Self::process_event_remove(event),
            notify::EventKind::Access(_) | notify::EventKind::Any | notify::EventKind::Other => {
                unreachable!("filtered out before hand")
            }
        }
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(level = "trace"))]
    fn process_event_create(event: DebouncedEvent) -> Vec<event::Event> {
        let notify::EventKind::Create(kind) = &event.kind else {
            panic!("invalid event kind");
        };

        let [path] = &event.paths[..] else {
            panic!("invalid paths");
        };

        match kind {
            notify::event::CreateKind::File => {
                vec![event::File::Created(path.clone()).into()]
            }
            notify::event::CreateKind::Folder => {
                vec![event::Folder::Created(path.clone()).into()]
            }
            notify::event::CreateKind::Any | notify::event::CreateKind::Other => {
                if path.is_file() {
                    vec![event::File::Created(path.clone()).into()]
                } else if path.is_dir() {
                    vec![event::Folder::Created(path.clone()).into()]
                } else {
                    vec![]
                }
            }
        }
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(level = "trace"))]
    fn process_event_modify(event: DebouncedEvent) -> Vec<event::Event> {
        let notify::EventKind::Modify(kind) = event.kind else {
            panic!("invalid event kind");
        };

        match kind {
            notify::event::ModifyKind::Name(_) => Self::process_event_modify_name(event),
            notify::event::ModifyKind::Any
            | notify::event::ModifyKind::Data(_)
            | notify::event::ModifyKind::Other => Self::process_event_modify_content(event),
            notify::event::ModifyKind::Metadata(_) => vec![],
        }
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(level = "trace"))]
    fn process_event_modify_name(event: DebouncedEvent) -> Vec<event::Event> {
        let notify::EventKind::Modify(notify::event::ModifyKind::Name(kind)) = event.kind else {
            panic!("invalid event kind");
        };

        match kind {
            notify::event::RenameMode::Both => {
                let [from, to] = &event.paths[..] else {
                    panic!("invalid paths");
                };

                if to.is_file() {
                    vec![
                        event::File::Renamed {
                            from: from.clone(),
                            to: to.clone(),
                        }
                        .into(),
                    ]
                } else if to.is_dir() {
                    vec![
                        event::Folder::Renamed {
                            from: from.clone(),
                            to: to.clone(),
                        }
                        .into(),
                    ]
                } else {
                    vec![]
                }
            }
            notify::event::RenameMode::To => {
                let [path] = &event.paths[..] else {
                    panic!("invalid paths");
                };

                if path.is_file() {
                    vec![event::File::Created(path.clone()).into()]
                } else if path.is_dir() {
                    vec![event::Folder::Created(path.clone()).into()]
                } else {
                    vec![]
                }
            }
            notify::event::RenameMode::From => {
                let [path] = &event.paths[..] else {
                    panic!("invalid paths");
                };

                vec![event::Any::Removed(path.clone()).into()]
            }
            notify::event::RenameMode::Any | notify::event::RenameMode::Other => todo!(),
        }
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(level = "trace"))]
    fn process_event_modify_content(event: DebouncedEvent) -> Vec<event::Event> {
        let notify::EventKind::Modify(kind) = event.kind else {
            panic!("invalid event kind");
        };
        assert_matches!(
            kind,
            notify::event::ModifyKind::Any
                | notify::event::ModifyKind::Data(_)
                | notify::event::ModifyKind::Other
        );

        let [path] = &event.paths[..] else {
            panic!("invalid paths");
        };

        if path.is_file() {
            vec![event::File::Modified(path.clone()).into()]
        } else {
            vec![]
        }
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(level = "trace"))]
    fn process_event_remove(event: DebouncedEvent) -> Vec<event::Event> {
        let notify::EventKind::Remove(kind) = event.kind else {
            panic!("invalid event kind");
        };

        let [path] = &event.paths[..] else {
            panic!("invalid paths");
        };

        match kind {
            notify::event::RemoveKind::File => vec![event::File::Removed(path.clone()).into()],
            notify::event::RemoveKind::Folder => vec![event::Folder::Removed(path.clone()).into()],
            notify::event::RemoveKind::Any | notify::event::RemoveKind::Other => {
                vec![event::Any::Removed(path.clone()).into()]
            }
        }
    }
}
