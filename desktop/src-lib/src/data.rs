use hermes_core as core;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[cfg(feature = "fs")]
use std::{fs, path::Path};

pub type Data = core::expr::Value;
pub type CellMap = BTreeMap<core::data::CellIndex, Data>;

#[derive(Serialize, Deserialize, Clone, Debug, derive_more::Deref)]
pub struct Spreadsheet {
    #[deref]
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

    pub fn iter_rows<'a>(&'a self) -> SpreadsheetRowIter<'a> {
        SpreadsheetRowIter::new(self)
    }
}

impl Spreadsheet {
    /// Sets the value of a cell.
    /// If a value already existed in the cell it is overwritten.
    pub fn set(&mut self, idx: core::data::CellIndex, value: Data) {
        if idx.row() >= self.size.0 {
            self.size.0 = idx.row() + 1;
        }
        if idx.col() >= self.size.1 {
            self.size.1 = idx.col() + 1;
        }

        self.cells.insert(idx, value);
    }

    /// Inserts a value into a cell.
    /// If a value already exists at that location the insert fails.
    pub fn insert(
        &mut self,
        idx: core::data::CellIndex,
        value: Data,
    ) -> Result<(), error::CellNotEmpty> {
        if self.cells.contains_key(&idx) {
            return Err(error::CellNotEmpty);
        }

        if idx.row() >= self.size.0 {
            self.size.0 = idx.row() + 1;
        }
        if idx.col() >= self.size.1 {
            self.size.1 = idx.col() + 1;
        }

        self.cells.insert(idx, value);
        Ok(())
    }
}

pub struct SpreadsheetRowIter<'a> {
    sheet: &'a Spreadsheet,
    rows: core::data::IndexType,
    cols: core::data::IndexType,
    next_row: core::data::IndexType,
}

impl<'a> SpreadsheetRowIter<'a> {
    pub fn new(sheet: &'a Spreadsheet) -> Self {
        let (rows, cols) = sheet.size();
        Self {
            sheet,
            rows,
            cols,
            next_row: 0,
        }
    }
}

impl<'a> std::iter::Iterator for SpreadsheetRowIter<'a> {
    type Item = Vec<&'a Data>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next_row >= self.rows {
            return None;
        }

        let mut row = vec![&Data::Empty; self.cols as usize];
        for (idx, data) in self.sheet.cells.iter() {
            if idx.row() == self.next_row {
                row[idx.col() as usize] = data;
            }
        }
        self.next_row += 1;
        Some(row)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Csv {
    pub sheet: Spreadsheet,
}

#[cfg(feature = "fs")]
impl Csv {
    pub fn from_csv_reader(reader: csv::Reader<fs::File>) -> Result<Self, error::LoadCsv> {
        reader.try_into()
    }

    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self, error::LoadCsv> {
        let mut cells = CellMap::new();
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_path(path)?;

        reader.try_into()
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), error::SaveCsv> {
        let tmp_file =
            tempfile::NamedTempFile::new().map_err(|err| error::SaveCsv::Io(err.kind()))?;
        let mut wtr = csv::Writer::from_path(tmp_file.path())?;
        for row in self.sheet.iter_rows() {
            let row_str = row
                .into_iter()
                .map(|value| value.to_string())
                .collect::<Vec<_>>();

            wtr.write_record(row_str)?;
        }

        fs::rename(tmp_file.path(), path).map_err(|err| error::SaveCsv::Io(err.kind()))?;
        Ok(())
    }
}

#[cfg(feature = "fs")]
impl TryFrom<csv::Reader<fs::File>> for Csv {
    type Error = error::LoadCsv;

    fn try_from(mut reader: csv::Reader<fs::File>) -> Result<Self, Self::Error> {
        let mut cells = CellMap::new();
        for (row, result) in reader.records().enumerate() {
            let record = result.expect("result is valid");
            if row > core::data::IndexType::MAX.into() {
                return Err(error::LoadCsv::DataTooLarge);
            }

            for (col, value) in record.into_iter().enumerate() {
                if col > core::data::IndexType::MAX.into() {
                    return Err(error::LoadCsv::DataTooLarge);
                }

                let idx = (row as core::data::IndexType, col as core::data::IndexType);
                let value = str_value_to_data(value);
                let _ = cells.insert(idx.into(), value);
            }
        }

        let sheet = Spreadsheet::from_cells(cells);
        Ok(Self { sheet })
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Workbook {
    sheets: Vec<(String, Spreadsheet)>,
}

impl Workbook {
    pub fn sheet_names(&self) -> Vec<&String> {
        self.sheets.iter().map(|(name, _)| name).collect()
    }

    pub fn get_sheet(&self, idx: usize) -> Option<&Spreadsheet> {
        self.sheets.get(idx).map(|(_, sheet)| sheet)
    }

    pub fn get_sheet_mut(&mut self, idx: usize) -> Option<&mut Spreadsheet> {
        self.sheets.get_mut(idx).map(|(_, sheet)| sheet)
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
    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self, error::LoadExcel> {
        // TODO: currently just a placeholder
        let cells = CellMap::new();
        let sheet = Spreadsheet::from_cells(cells);
        Ok(Self {
            sheets: vec![("".into(), sheet)],
        })
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, derive_more::From)]
pub enum Dataset {
    Csv(Csv),
    Workbook(Workbook),
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

    #[derive(Debug)]
    pub struct CellNotEmpty;

    #[derive(Serialize, Deserialize, Debug, thiserror::Error, Clone, derive_more::From)]
    pub enum Save {
        #[error("error saving csv: {0}")]
        Csv(SaveCsv),
        #[error("error saving excel: {0}")]
        Excel(SaveExcel),
    }

    #[derive(Serialize, Deserialize, Debug, thiserror::Error, Clone, derive_more::From)]
    pub enum SaveCsv {
        #[error("{0}")]
        Io(#[serde(with = "io_error_serde::ErrorKind")] io::ErrorKind),
    }

    #[cfg(feature = "fs")]
    impl From<csv::Error> for SaveCsv {
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
    pub enum SaveExcel {
        #[error("{0}")]
        Io(#[serde(with = "io_error_serde::ErrorKind")] io::ErrorKind),
    }

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
        DataTooLarge,
    }

    #[cfg(feature = "fs")]
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
