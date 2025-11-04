use std::path::PathBuf;

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
    Updated,
}
