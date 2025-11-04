use crate::data;
use hermes_core as core;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, derive_more::From, Clone, Debug)]
pub enum WorkspaceOrder {
    Create,
    Update(Update),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Update {
    pub path: PathBuf,
    pub updates: Updates,
}

#[derive(Serialize, Deserialize, derive_more::From, Clone, Debug)]
pub enum Updates {
    Csv(Vec<UpdateCsv>),
    Workbook(Vec<UpdateWorkbook>),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UpdateCsv {
    pub row: core::data::IndexType,
    pub col: core::data::IndexType,
    pub value: core::expr::Value,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UpdateWorkbook {
    pub sheet: core::data::IndexType,
    pub row: core::data::IndexType,
    pub col: core::data::IndexType,
    pub value: core::expr::Value,
}

pub mod error {
    use crate::data;
    use serde::{Deserialize, Serialize};
    use std::io;

    #[derive(Serialize, Deserialize, Clone, Debug)]

    pub enum WorkspaceOrder {
        /// The task could not be completed.
        TaskNotCompleted,
        /// File could not be opened.
        OpenFile(#[serde(with = "io_error_serde::ErrorKind")] io::ErrorKind),
        /// File could not be saved.
        Save(#[serde(with = "io_error_serde::ErrorKind")] io::ErrorKind),
    }

    impl From<data::error::LoadCsv> for WorkspaceOrder {
        fn from(value: data::error::LoadCsv) -> Self {
            match value {
                data::error::LoadCsv::Io(err) => Self::OpenFile(err),
                data::error::LoadCsv::DataTooLarge => todo!(),
            }
        }
    }

    impl From<data::error::SaveCsv> for WorkspaceOrder {
        fn from(value: data::error::SaveCsv) -> Self {
            match value {
                data::error::SaveCsv::Io(err) => Self::Save(err),
            }
        }
    }
}
