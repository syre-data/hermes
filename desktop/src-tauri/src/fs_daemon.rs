use crossbeam::channel;
use notify_debouncer_full::{DebounceEventResult, new_debouncer};
use std::{path::PathBuf, time::Duration};

pub mod event {
    pub struct Event {
        paths: Vec<PathBuf>,
        resource: FsResource,
        kind: EventKind,
    }

    pub enum FsResource {
        Folder,
        File,
    }

    pub enum EventKind {
        Created,
        Removed,
        Renamed,
        Moved,
    }
}

pub struct Builder {
    root: PathBuf,
    tx: crossbeam::channel::Sender<event::Event>,
}

impl Builder {
    pub fn new(root: PathBuf, tx: channel::Sender<event::Event>) -> Self {
        Self { root, tx }
    }

    fn run(&self) {
        // Select recommended watcher for debouncer.
        // Using a callback here, could also be a channel.
        let mut debouncer = new_debouncer(
            Duration::from_millis(200),
            None,
            |result: DebounceEventResult| match result {
                Ok(events) => events.iter().for_each(|event| println!("{event:?}")),
                Err(errors) => errors.iter().for_each(|error| println!("{error:?}")),
            },
        )
        .unwrap();

        // Add a path to be watched. All files and directories at that path and
        // below will be monitored for changes.
        debouncer.watch(".", RecursiveMode::Recursive).unwrap();
    }
}
