use crate::{ResourceId, data};
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

impl Update {
    pub fn formulas(&self) -> Vec<ResourceId> {
        match &self.updates {
            Updates::Csv(updates) => updates
                .iter()
                .map(|update| update.formula())
                .cloned()
                .collect(),
            Updates::Workbook(updates) => updates
                .iter()
                .map(|update| update.formula())
                .cloned()
                .collect(),
        }
    }
}

#[derive(Serialize, Deserialize, derive_more::From, Clone, Debug)]
pub enum Updates {
    Csv(Vec<UpdateCsv>),
    Workbook(Vec<UpdateWorkbook>),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UpdateCsv {
    formula: ResourceId,
    row: core::data::IndexType,
    col: core::data::IndexType,
    value: core::expr::Value,
}

impl UpdateCsv {
    pub fn new(
        formula: ResourceId,
        row: core::data::IndexType,
        col: core::data::IndexType,
        value: core::expr::Value,
    ) -> Self {
        Self {
            formula,
            row,
            col,
            value,
        }
    }

    /// # Returns
    /// Tuple of `(formula, row, col, value)`.
    pub fn into_parts(
        self,
    ) -> (
        ResourceId,
        core::data::IndexType,
        core::data::IndexType,
        core::expr::Value,
    ) {
        let Self {
            formula,
            row,
            col,
            value,
        } = self;
        (formula, row, col, value)
    }

    pub fn formula(&self) -> &ResourceId {
        &self.formula
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UpdateWorkbook {
    formula: ResourceId,
    sheet: core::data::IndexType,
    row: core::data::IndexType,
    col: core::data::IndexType,
    value: core::expr::Value,
}

impl UpdateWorkbook {
    pub fn new(
        formula: ResourceId,
        sheet: core::data::IndexType,
        row: core::data::IndexType,
        col: core::data::IndexType,
        value: core::expr::Value,
    ) -> Self {
        Self {
            formula,
            sheet,
            row,
            col,
            value,
        }
    }

    pub fn formula(&self) -> &ResourceId {
        &self.formula
    }
}

pub mod error {
    use crate::{ResourceId, data};
    use serde::{Deserialize, Serialize};
    use std::{io, path::PathBuf};

    #[derive(Serialize, Deserialize, Clone, Debug)]
    pub struct WorkspaceOrder {
        formulas: Vec<ResourceId>,
        kind: WorkspaceOrderKind,
    }

    impl WorkspaceOrder {
        pub fn new(formulas: Vec<ResourceId>, kind: WorkspaceOrderKind) -> Self {
            Self { formulas, kind }
        }

        pub fn into_parts(self) -> (Vec<ResourceId>, WorkspaceOrderKind) {
            let Self { formulas, kind } = self;
            (formulas, kind)
        }

        pub fn formulas(&self) -> &Vec<ResourceId> {
            &self.formulas
        }
    }

    #[derive(Serialize, Deserialize, Clone, Debug)]

    pub enum WorkspaceOrderKind {
        /// The task could not be completed.
        TaskNotCompleted,
        /// File could not be opened.
        OpenFile {
            path: PathBuf,
            #[serde(with = "io_error_serde::ErrorKind")]
            error: io::ErrorKind,
        },
        /// File could not be saved.
        Save {
            path: PathBuf,
            #[serde(with = "io_error_serde::ErrorKind")]
            error: io::ErrorKind,
        },
    }
}
