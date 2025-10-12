use crate::utils;
use std::fmt;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

pub type IndexType = u16;
pub const SHEET_DELIMETER: char = '!';
pub const REF_MODE_SIGIL: char = '$';

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefMode {
    Relative,
    Absolute,
}

#[derive(Clone, Debug, derive_more::From, PartialEq, Eq)]
pub enum SheetIndex {
    Index(IndexType),
    Label(String),
}

#[derive(Clone, Debug, derive_more::From, PartialEq, Eq)]
pub enum SheetRef {
    Relative,
    Absolute(SheetIndex),
}

impl From<Option<SheetIndex>> for SheetRef {
    fn from(value: Option<SheetIndex>) -> Self {
        match value {
            Some(idx) => Self::Absolute(idx),
            None => Self::Relative,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CellRef {
    pub sheet: SheetRef,
    pub row: IndexType,
    pub col: IndexType,
    pub col_mode: RefMode,
    pub row_mode: RefMode,
}

impl CellRef {
    pub fn dynamic(row: impl Into<IndexType>, col: impl Into<IndexType>) -> Self {
        Self {
            sheet: SheetRef::Relative,
            row: row.into(),
            col: col.into(),
            col_mode: RefMode::Relative,
            row_mode: RefMode::Relative,
        }
    }

    pub fn dynamic_with_sheet(
        row: impl Into<IndexType>,
        col: impl Into<IndexType>,
        sheet: impl Into<SheetIndex>,
    ) -> Self {
        Self {
            sheet: SheetRef::Absolute(sheet.into()),
            row: row.into(),
            col: col.into(),
            col_mode: RefMode::Relative,
            row_mode: RefMode::Relative,
        }
    }

    pub fn col_absolute(row: impl Into<IndexType>, col: impl Into<IndexType>) -> Self {
        Self {
            sheet: SheetRef::Relative,
            row: row.into(),
            col: col.into(),
            col_mode: RefMode::Absolute,
            row_mode: RefMode::Relative,
        }
    }

    pub fn col_absolute_with_sheet(
        row: impl Into<IndexType>,
        col: impl Into<IndexType>,
        sheet: impl Into<SheetIndex>,
    ) -> Self {
        Self {
            sheet: SheetRef::Absolute(sheet.into()),
            row: row.into(),
            col: col.into(),
            col_mode: RefMode::Absolute,
            row_mode: RefMode::Relative,
        }
    }

    pub fn row_absolute(row: impl Into<IndexType>, col: impl Into<IndexType>) -> Self {
        Self {
            sheet: SheetRef::Relative,
            row: row.into(),
            col: col.into(),
            col_mode: RefMode::Relative,
            row_mode: RefMode::Absolute,
        }
    }

    pub fn row_absolute_with_sheet(
        row: impl Into<IndexType>,
        col: impl Into<IndexType>,
        sheet: impl Into<SheetIndex>,
    ) -> Self {
        Self {
            sheet: SheetRef::Absolute(sheet.into()),
            row: row.into(),
            col: col.into(),
            col_mode: RefMode::Relative,
            row_mode: RefMode::Absolute,
        }
    }

    pub fn aboslute(row: impl Into<IndexType>, col: impl Into<IndexType>) -> Self {
        Self {
            sheet: SheetRef::Relative,
            row: row.into(),
            col: col.into(),
            col_mode: RefMode::Absolute,
            row_mode: RefMode::Absolute,
        }
    }

    pub fn absolute_with_sheet(
        row: impl Into<IndexType>,
        col: impl Into<IndexType>,
        sheet: impl Into<SheetIndex>,
    ) -> Self {
        Self {
            sheet: SheetRef::Absolute(sheet.into()),
            row: row.into(),
            col: col.into(),
            col_mode: RefMode::Absolute,
            row_mode: RefMode::Absolute,
        }
    }
}

impl CellRef {
    /// Parse a string.
    /// Valid cell indexes have the form `[<sheet>!]<a-z>[<a-z>]\d+`.
    /// e.g. `a1`, `b5`, `d40`, `bf300`, `sheet1!s4`, `my_sheet!bf232`, `0!a4`.
    /// Sheet labels and letters are case insensitive.
    pub fn from_str(value: impl AsRef<str>) -> Option<Self> {
        let value = value.as_ref();
        let (sheet, cell) = match value.split_once(SHEET_DELIMETER) {
            Some((sheet, cell)) => (Some(sheet), cell),
            None => (None, value),
        };

        let sheet = sheet.map(|sheet| {
            if let Some(sheet) = sheet.parse::<IndexType>().ok() {
                SheetIndex::Index(sheet)
            } else {
                SheetIndex::Label(sheet.to_string())
            }
        });

        let mut col = vec![];
        let mut row = vec![];
        let mut col_mode = RefMode::Relative;
        let mut row_mode = RefMode::Relative;
        let mut chars = cell.chars();

        let mut next = chars.next()?;
        if next == REF_MODE_SIGIL {
            col_mode = RefMode::Absolute;
            next = chars.next()?;
        }
        while next.is_ascii_alphabetic() {
            col.push(next);
            next = chars.next()?;
        }

        if next == REF_MODE_SIGIL {
            row_mode = RefMode::Absolute;
            next = chars.next()?;
        }
        if !next.is_ascii_digit() {
            return None;
        }
        row.push(next);
        while let Some(next) = chars.next() {
            if !next.is_ascii_digit() {
                return None;
            }
            row.push(next);
        }

        let col = col.into_iter().collect::<String>();
        let row = row
            .into_iter()
            .collect::<String>()
            .parse::<IndexType>()
            .expect("row is a valid index");
        let col = utils::col_to_index(col)?;
        let row = utils::row_to_index(row)?;

        Some(Self {
            sheet: sheet.into(),
            col,
            row,
            col_mode,
            row_mode,
        })
    }
}

impl fmt::Display for CellRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            sheet,
            row,
            col,
            col_mode,
            row_mode,
        } = self;
        if let SheetRef::Absolute(sheet) = sheet {
            match sheet {
                SheetIndex::Index(idx) => write!(f, "{idx}")?,
                SheetIndex::Label(idx) => write!(f, "{idx}")?,
            }
            write!(f, "{SHEET_DELIMETER}")?;
        }

        if matches!(col_mode, RefMode::Absolute) {
            write!(f, "{REF_MODE_SIGIL}")?;
        }
        write!(f, "{}", &utils::index_to_col(*col))?;

        if matches!(row_mode, RefMode::Absolute) {
            write!(f, "{REF_MODE_SIGIL}")?;
        }
        write!(f, "{}", &utils::index_to_row(*row))?;

        Ok(())
    }
}

#[derive(Ord, Eq, Clone, Debug)]
// #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CellIndex {
    row: IndexType,
    col: IndexType,
}

impl CellIndex {
    pub fn new(row: impl Into<IndexType>, col: impl Into<IndexType>) -> Self {
        Self {
            row: row.into(),
            col: col.into(),
        }
    }

    pub fn row(&self) -> IndexType {
        self.row
    }

    pub fn col(&self) -> IndexType {
        self.col
    }
}

impl<T, U> From<(T, U)> for CellIndex
where
    T: Into<IndexType>,
    U: Into<IndexType>,
{
    fn from((row, col): (T, U)) -> Self {
        Self {
            row: row.into(),
            col: col.into(),
        }
    }
}

impl PartialEq for CellIndex {
    fn eq(&self, other: &Self) -> bool {
        self.row == other.row && self.col == other.col
    }
}

impl PartialOrd for CellIndex {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        use std::cmp::Ordering;

        Some(
            match self
                .row
                .partial_cmp(&other.row)
                .expect("rows to be comparable")
            {
                Ordering::Equal => self
                    .col
                    .partial_cmp(&other.col)
                    .expect("cols to be comparable"),
                ord => ord,
            },
        )
    }
}

impl fmt::Display for CellIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}{}",
            utils::index_to_col(self.col),
            utils::index_to_row(self.row)
        )
    }
}

#[cfg(feature = "serde")]
impl Serialize for CellIndex {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&format!("({},{})", self.row, self.col))
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for CellIndex {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use std::fmt;

        struct IndexVisitor;
        impl<'de> serde::de::Visitor<'de> for IndexVisitor {
            type Value = CellIndex;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("string of form `(<row>,<col>)`")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if !v.starts_with('(') || !v.ends_with(')') {
                    return Err(E::custom("index not enclosed in parentheses"));
                }

                let Some((row, col)) = v[1..v.len() - 1].split_once(",") else {
                    return Err(E::custom("invalid index body, could not split at comma"));
                };

                let row = row
                    .trim()
                    .parse::<IndexType>()
                    .map_err(|err| E::custom("could not parse row: {err:?}"))?;

                let col = col
                    .trim()
                    .parse::<IndexType>()
                    .map_err(|err| E::custom("could not parse col: {err:?}"))?;

                Ok(CellIndex { row, col })
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                self.visit_str(v.as_str())
            }
        }

        deserializer.deserialize_any(IndexVisitor)
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Range {
    /// Unbounded columnar input.
    Cols(Vec<IndexType>),

    /// Unbounded row input.
    Rows(Vec<IndexType>),

    /// Bounded input.
    Rect { start: CellIndex, end: CellIndex },
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn cell_ref_from_str() {
        assert_eq!(
            CellRef::from_str("a1"),
            Some(CellRef {
                sheet: SheetRef::Relative,
                col: 0,
                row: 0,
                col_mode: RefMode::Relative,
                row_mode: RefMode::Relative,
            })
        );
        assert_eq!(
            CellRef::from_str("b10"),
            Some(CellRef {
                sheet: SheetRef::Relative,
                col: 1,
                row: 9,
                col_mode: RefMode::Relative,
                row_mode: RefMode::Relative,
            })
        );
        assert_eq!(
            CellRef::from_str("ab2"),
            Some(CellRef {
                sheet: SheetRef::Relative,
                col: 27,
                row: 1,
                col_mode: RefMode::Relative,
                row_mode: RefMode::Relative,
            })
        );
        assert_eq!(
            CellRef::from_str("ac24"),
            Some(CellRef {
                sheet: SheetRef::Relative,
                col: 28,
                row: 23,
                col_mode: RefMode::Relative,
                row_mode: RefMode::Relative,
            })
        );

        assert_eq!(
            CellRef::from_str("$a1"),
            Some(CellRef {
                sheet: SheetRef::Relative,
                col: 0,
                row: 0,
                col_mode: RefMode::Absolute,
                row_mode: RefMode::Relative,
            })
        );
        assert_eq!(
            CellRef::from_str("$b10"),
            Some(CellRef {
                sheet: SheetRef::Relative,
                col: 1,
                row: 9,
                col_mode: RefMode::Absolute,
                row_mode: RefMode::Relative,
            })
        );
        assert_eq!(
            CellRef::from_str("$ab2"),
            Some(CellRef {
                sheet: SheetRef::Relative,
                col: 27,
                row: 1,
                col_mode: RefMode::Absolute,
                row_mode: RefMode::Relative,
            })
        );
        assert_eq!(
            CellRef::from_str("$ac24"),
            Some(CellRef {
                sheet: SheetRef::Relative,
                col: 28,
                row: 23,
                col_mode: RefMode::Absolute,
                row_mode: RefMode::Relative,
            })
        );

        assert_eq!(
            CellRef::from_str("a$1"),
            Some(CellRef {
                sheet: SheetRef::Relative,
                col: 0,
                row: 0,
                col_mode: RefMode::Relative,
                row_mode: RefMode::Absolute,
            })
        );
        assert_eq!(
            CellRef::from_str("b$10"),
            Some(CellRef {
                sheet: SheetRef::Relative,
                col: 1,
                row: 9,
                col_mode: RefMode::Relative,
                row_mode: RefMode::Absolute,
            })
        );
        assert_eq!(
            CellRef::from_str("ab$2"),
            Some(CellRef {
                sheet: SheetRef::Relative,
                col: 27,
                row: 1,
                col_mode: RefMode::Relative,
                row_mode: RefMode::Absolute,
            })
        );
        assert_eq!(
            CellRef::from_str("ac$24"),
            Some(CellRef {
                sheet: SheetRef::Relative,
                col: 28,
                row: 23,
                col_mode: RefMode::Relative,
                row_mode: RefMode::Absolute,
            })
        );

        assert_eq!(
            CellRef::from_str("$a$1"),
            Some(CellRef {
                sheet: SheetRef::Relative,
                col: 0,
                row: 0,
                col_mode: RefMode::Absolute,
                row_mode: RefMode::Absolute,
            })
        );
        assert_eq!(
            CellRef::from_str("$b$10"),
            Some(CellRef {
                sheet: SheetRef::Relative,
                col: 1,
                row: 9,
                col_mode: RefMode::Absolute,
                row_mode: RefMode::Absolute,
            })
        );
        assert_eq!(
            CellRef::from_str("$ab$2"),
            Some(CellRef {
                sheet: SheetRef::Relative,
                col: 27,
                row: 1,
                col_mode: RefMode::Absolute,
                row_mode: RefMode::Absolute,
            })
        );
        assert_eq!(
            CellRef::from_str("$ac$24"),
            Some(CellRef {
                sheet: SheetRef::Relative,
                col: 28,
                row: 23,
                col_mode: RefMode::Absolute,
                row_mode: RefMode::Absolute,
            })
        );

        assert_eq!(
            CellRef::from_str("sheet!$a$1"),
            Some(CellRef {
                sheet: SheetRef::Absolute(SheetIndex::Label("sheet".to_string())),
                col: 0,
                row: 0,
                col_mode: RefMode::Absolute,
                row_mode: RefMode::Absolute,
            })
        );
        assert_eq!(
            CellRef::from_str("0!$b$10"),
            Some(CellRef {
                sheet: SheetRef::Absolute(SheetIndex::Index(0)),
                col: 1,
                row: 9,
                col_mode: RefMode::Absolute,
                row_mode: RefMode::Absolute,
            })
        );
        assert_eq!(
            CellRef::from_str("sheet!$ab$2"),
            Some(CellRef {
                sheet: SheetRef::Absolute(SheetIndex::Label("sheet".to_string())),
                col: 27,
                row: 1,
                col_mode: RefMode::Absolute,
                row_mode: RefMode::Absolute,
            })
        );
        assert_eq!(
            CellRef::from_str("2!$ac$24"),
            Some(CellRef {
                sheet: SheetRef::Absolute(SheetIndex::Index(2)),
                col: 28,
                row: 23,
                col_mode: RefMode::Absolute,
                row_mode: RefMode::Absolute,
            })
        );

        assert!(CellRef::from_str("4").is_none());
        assert!(CellRef::from_str("a").is_none());
        assert!(CellRef::from_str("acb24").is_none());
        assert!(CellRef::from_str("a2c").is_none());
        assert!(CellRef::from_str("$$a2").is_none());
        assert!(CellRef::from_str("a2$").is_none());
        assert!(CellRef::from_str("a2!sheet").is_none());
        assert!(CellRef::from_str("sheet!a2!").is_none());
        assert!(CellRef::from_str("sheet$a2").is_none());
    }

    #[test]
    fn cell_ref_as_str() {
        let cell = CellRef {
            sheet: SheetRef::Relative,
            col: 0,
            row: 0,
            col_mode: RefMode::Relative,
            row_mode: RefMode::Relative,
        };
        assert_eq!(format!("{cell}"), "A1");

        let cell = CellRef {
            sheet: SheetRef::Relative,
            col: 1,
            row: 9,
            col_mode: RefMode::Relative,
            row_mode: RefMode::Relative,
        };
        assert_eq!(format!("{cell}"), "B10");

        let cell = CellRef {
            sheet: SheetRef::Relative,
            col: 27,
            row: 1,
            col_mode: RefMode::Relative,
            row_mode: RefMode::Relative,
        };
        assert_eq!(format!("{cell}"), "AB2");

        let cell = CellRef {
            sheet: SheetRef::Relative,
            col: 28,
            row: 23,
            col_mode: RefMode::Relative,
            row_mode: RefMode::Relative,
        };
        assert_eq!(format!("{cell}"), "AC24");

        let cell = CellRef {
            sheet: SheetRef::Relative,
            col: 0,
            row: 0,
            col_mode: RefMode::Absolute,
            row_mode: RefMode::Relative,
        };
        assert_eq!(format!("{cell}"), "$A1");

        let cell = CellRef {
            sheet: SheetRef::Relative,
            col: 1,
            row: 9,
            col_mode: RefMode::Absolute,
            row_mode: RefMode::Relative,
        };
        assert_eq!(format!("{cell}"), "$B10");

        let cell = CellRef {
            sheet: SheetRef::Relative,
            col: 27,
            row: 1,
            col_mode: RefMode::Absolute,
            row_mode: RefMode::Relative,
        };
        assert_eq!(format!("{cell}"), "$AB2");

        let cell = CellRef {
            sheet: SheetRef::Relative,
            col: 28,
            row: 23,
            col_mode: RefMode::Absolute,
            row_mode: RefMode::Relative,
        };
        assert_eq!(format!("{cell}"), "$AC24");

        let cell = CellRef {
            sheet: SheetRef::Relative,
            col: 0,
            row: 0,
            col_mode: RefMode::Relative,
            row_mode: RefMode::Absolute,
        };
        assert_eq!(format!("{cell}"), "A$1");

        let cell = CellRef {
            sheet: SheetRef::Relative,
            col: 1,
            row: 9,
            col_mode: RefMode::Relative,
            row_mode: RefMode::Absolute,
        };
        assert_eq!(format!("{cell}"), "B$10");

        let cell = CellRef {
            sheet: SheetRef::Relative,
            col: 27,
            row: 1,
            col_mode: RefMode::Relative,
            row_mode: RefMode::Absolute,
        };
        assert_eq!(format!("{cell}"), "AB$2");

        let cell = CellRef {
            sheet: SheetRef::Relative,
            col: 28,
            row: 23,
            col_mode: RefMode::Relative,
            row_mode: RefMode::Absolute,
        };
        assert_eq!(format!("{cell}"), "AC$24");

        let cell = CellRef {
            sheet: SheetRef::Relative,
            col: 0,
            row: 0,
            col_mode: RefMode::Absolute,
            row_mode: RefMode::Absolute,
        };
        assert_eq!(format!("{cell}"), "$A$1");

        let cell = CellRef {
            sheet: SheetRef::Relative,
            col: 1,
            row: 9,
            col_mode: RefMode::Absolute,
            row_mode: RefMode::Absolute,
        };
        assert_eq!(format!("{cell}"), "$B$10");

        let cell = CellRef {
            sheet: SheetRef::Relative,
            col: 27,
            row: 1,
            col_mode: RefMode::Absolute,
            row_mode: RefMode::Absolute,
        };
        assert_eq!(format!("{cell}"), "$AB$2");

        let cell = CellRef {
            sheet: SheetRef::Relative,
            col: 28,
            row: 23,
            col_mode: RefMode::Absolute,
            row_mode: RefMode::Absolute,
        };
        assert_eq!(format!("{cell}"), "$AC$24");

        let cell = CellRef {
            sheet: SheetRef::Absolute(SheetIndex::Label("sheet".to_string())),
            col: 0,
            row: 0,
            col_mode: RefMode::Absolute,
            row_mode: RefMode::Absolute,
        };
        assert_eq!(format!("{cell}"), "sheet!$A$1");

        let cell = CellRef {
            sheet: SheetRef::Absolute(SheetIndex::Index(0)),
            col: 1,
            row: 9,
            col_mode: RefMode::Absolute,
            row_mode: RefMode::Absolute,
        };
        assert_eq!(format!("{cell}"), "0!$B$10");

        let cell = CellRef {
            sheet: SheetRef::Absolute(SheetIndex::Label("sheet".to_string())),
            col: 27,
            row: 1,
            col_mode: RefMode::Absolute,
            row_mode: RefMode::Absolute,
        };
        assert_eq!(format!("{cell}"), "sheet!$AB$2");

        let cell = CellRef {
            sheet: SheetRef::Absolute(SheetIndex::Index(2)),
            col: 28,
            row: 23,
            col_mode: RefMode::Absolute,
            row_mode: RefMode::Absolute,
        };
        assert_eq!(format!("{cell}"), "2!$AC$24");
    }
}
