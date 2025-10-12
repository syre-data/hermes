use hermes_core as core;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[cfg(feature = "fs")]
use std::path::Path;

pub type Data = calamine::Data;
pub type CellMap = BTreeMap<core::data::CellIndex, Data>;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Spreadsheet {
    cells: CellMap,

    /// Number of (rows, cols).
    /// Each is the max index value of their respective value contained.
    size: (core::data::IndexType, core::data::IndexType),
}

impl Spreadsheet {
    pub fn new() -> Self {
        Self::from_cells(CellMap::new())
    }

    pub fn from_cells(cells: CellMap) -> Self {
        let size = if cells.is_empty() {
            (0, 0)
        } else {
            let mut max_row = 0;
            let mut max_col = 0;
            for idx in cells.keys() {
                if idx.row() > max_row {
                    max_row = idx.row()
                }
                if idx.col() > max_col {
                    max_col = idx.col()
                }
            }
            (max_row + 1, max_col + 1)
        };

        Self { cells, size }
    }

    /// Number of (rows, cols).
    pub fn size(&self) -> (core::data::IndexType, core::data::IndexType) {
        self.size
    }

    pub fn cells(&self) -> &CellMap {
        &self.cells
    }

    pub fn is_empty(&self) -> bool {
        (0, 0) == self.size
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Workbook {
    sheets: Vec<(String, Spreadsheet)>,
    kind: WorkbookKind,
}

impl Workbook {
    pub fn kind(&self) -> WorkbookKind {
        self.kind
    }

    pub fn sheet_names(&self) -> Vec<&String> {
        self.sheets.iter().map(|(name, _)| name).collect()
    }

    pub fn get_sheet(&self, idx: usize) -> Option<&Spreadsheet> {
        self.sheets.get(idx).map(|(_, sheet)| sheet)
    }

    pub fn is_empty(&self) -> bool {
        self.sheets.is_empty()
    }

    pub fn sheets(&self) -> &Vec<(String, Spreadsheet)> {
        &self.sheets
    }
}

#[cfg(feature = "fs")]
impl Workbook {
    pub fn load_csv(path: impl AsRef<Path>) -> Result<Self, error::LoadCsv> {
        let mut cells = CellMap::new();
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_path(path)?;

        for (row, result) in reader.records().enumerate() {
            let record = result.expect("result is valid");
            if row > core::data::IndexType::MAX.into() {
                return Err(error::LoadCsv::TooLarge);
            }

            for (col, value) in record.into_iter().enumerate() {
                if col > core::data::IndexType::MAX.into() {
                    return Err(error::LoadCsv::TooLarge);
                }

                let idx = (row as core::data::IndexType, col as core::data::IndexType);
                let value = str_value_to_data(value);
                let _ = cells.insert(idx.into(), value);
            }
        }

        let sheet = Spreadsheet::from_cells(cells);
        Ok(Self {
            sheets: vec![("Sheet1".into(), sheet)],
            kind: WorkbookKind::Csv,
        })
    }

    pub fn load_excel(path: impl AsRef<Path>) -> Result<Self, error::LoadExcel> {
        let cells = CellMap::new();
        let sheet = Spreadsheet::from_cells(cells);
        Ok(Self {
            sheets: vec![("".into(), sheet)],
            kind: WorkbookKind::Workbook,
        })
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum WorkbookKind {
    Csv,
    Workbook,
}

fn str_value_to_data(value: &str) -> Data {
    if let Ok(value) = value.parse::<i64>() {
        Data::Int(value)
    } else if let Ok(value) = value.parse::<f64>() {
        Data::Float(value)
    } else if value.to_ascii_lowercase() == "true" {
        Data::Bool(true)
    } else if value.to_ascii_lowercase() == "false" {
        Data::Bool(false)
    } else {
        Data::String(value.to_string())
    }
}

pub mod error {
    use serde::{Deserialize, Serialize};
    use std::io;

    #[derive(Serialize, Deserialize, Debug, thiserror::Error, Clone, derive_more::From)]
    pub enum Load {
        #[error("invalid file type")]
        InvalidFileType,
        #[error("error loading csv: {0}")]
        Csv(LoadCsv),
        #[error("error loading excel: {0}")]
        Excel(LoadExcel),
    }

    #[derive(Serialize, Deserialize, Debug, thiserror::Error, Clone, derive_more::From)]
    pub enum LoadCsv {
        #[error("{0}")]
        Io(#[serde(with = "io_error_serde::ErrorKind")] io::ErrorKind),
        #[error("data is too large")]
        TooLarge,
    }

    impl From<csv::Error> for LoadCsv {
        fn from(value: csv::Error) -> Self {
            use csv::ErrorKind;

            match value.kind() {
                ErrorKind::Io(error) => Self::Io(error.kind()),
                ErrorKind::Utf8 { pos, err } => todo!(),
                ErrorKind::UnequalLengths {
                    pos,
                    expected_len,
                    len,
                } => todo!(),
                ErrorKind::Seek => todo!(),
                ErrorKind::Serialize(_) => todo!(),
                ErrorKind::Deserialize { pos, err } => todo!(),
                _ => todo!(),
            }
        }
    }

    #[derive(Serialize, Deserialize, Debug, thiserror::Error, Clone, derive_more::From)]
    pub enum LoadExcel {
        #[error("{0}")]
        Io(#[serde(with = "io_error_serde::ErrorKind")] io::ErrorKind),
    }
}
