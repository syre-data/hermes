use crate::event;
use notify_debouncer_full::{DebounceEventResult, Debouncer, FileIdMap};
use std::path::PathBuf;

const DEBOUNCE_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(100);

pub type EventSender = tokio::sync::mpsc::UnboundedSender<event::Event>;
pub type EventReceiver = tokio::sync::mpsc::UnboundedReceiver<event::Event>;
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
            tracing::trace!("waiting for command or event");
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
    fn handle_file_system_events(&self, events: DebounceEventResult) {
        #[cfg(feature = "tracing")]
        tracing::trace!(?events);
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
