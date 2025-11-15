use crate::{data, utils};
use serde::{Deserialize, Serialize};
use std::{ffi::OsString, path::PathBuf};

pub const EVENT_TOPIC: &str = "hermes://fs_event";

#[derive(Debug, Clone, Serialize, Deserialize, derive_more::From)]
pub enum Event {
    File(File),
    Folder(Folder),
    Any(Any),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum File {
    Created(PathBuf),
    Removed(PathBuf),
    Renamed {
        from: PathBuf,
        #[serde(with = "utils::serialize_os_string")]
        to: OsString,
    },
    Moved {
        from: PathBuf,
        to: PathBuf,
    },
    Modified {
        path: PathBuf,
        data: data::Dataset,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Folder {
    Created(PathBuf),
    Removed(PathBuf),
    Renamed {
        from: PathBuf,

        #[serde(with = "utils::serialize_os_string")]
        to: OsString,
    },
    Moved {
        from: PathBuf,
        to: PathBuf,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Any {
    Removed(PathBuf),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Error {
    LoadDataset {
        path: PathBuf,
        error: data::error::Load,
    },
}
