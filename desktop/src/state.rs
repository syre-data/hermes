use crate::{formula, message};
use hermes_core as core;
use hermes_desktop_lib as lib;
use leptos::prelude::*;
use std::{collections::BTreeMap, ffi::OsString, path::PathBuf, sync::Arc};

const CANVAS_ROWS_DEFAULT: core::data::IndexType = 100;
const CANVAS_COLS_DEFAULT: core::data::IndexType = 26;

pub trait FileResource {
    fn file(&self) -> &ResourceId;
}

#[derive(Clone, derive_more::Deref, Hash, PartialEq, Eq, Debug)]
pub struct ResourceId(uuid::Uuid);
impl ResourceId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

/// Abort handle used to cancel loading a workbook.
#[derive(Clone)]
pub struct LoadWorkbookActionAbortHandle(Option<Arc<ActionAbortHandle>>);
impl LoadWorkbookActionAbortHandle {
    pub fn new() -> Self {
        Self(None)
    }

    pub fn insert(&mut self, handle: ActionAbortHandle) {
        let _ = self.0.insert(Arc::new(handle));
    }

    pub fn take(&mut self) -> Option<ActionAbortHandle> {
        self.0
            .take()
            .map(|handle| Arc::into_inner(handle).expect("single owner"))
    }
}

/// Reactive owner for the workspace.
/// Use to hoist ownership when creating signals.
#[derive(Clone, derive_more::Deref)]
pub struct WorkspaceOwner(Owner);
impl WorkspaceOwner {
    pub fn with_current() -> Self {
        Self(Owner::current().expect("owner to exist"))
    }
}

#[derive(Clone)]
pub enum ActiveDataset {
    None,
    Some {
        id: ResourceId,
        active_cell: RwSignal<ActiveCell>,
    },
}

impl ActiveDataset {
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    pub fn is_some(&self) -> bool {
        matches!(self, Self::Some { .. })
    }

    /// Sets `self` to `Self::Some { id: <id>, active_cell: ActiveCell::None }`.
    pub fn insert(&mut self, id: ResourceId) {
        *self = Self::Some {
            id,
            active_cell: RwSignal::new(ActiveCell::None),
        }
    }

    pub fn take(&mut self) {
        *self = Self::None
    }

    pub fn as_ref(&self) -> Option<&ResourceId> {
        match self {
            Self::None => None,
            Self::Some { id, active_cell } => Some(id),
        }
    }

    pub fn map<F, T>(self, f: F) -> Option<T>
    where
        F: FnOnce(ResourceId) -> T,
    {
        match self {
            Self::None => None,
            Self::Some { id, .. } => Some(f(id)),
        }
    }
}

#[derive(Clone)]
pub enum ActiveCell {
    None,
    Some(core::data::CellIndex),
}

impl ActiveCell {
    pub fn is_some(&self) -> bool {
        matches!(self, Self::Some(_))
    }
}

/// `true` indicates the formula editor should be visible.
#[derive(Clone, Copy, Debug, derive_more::Deref)]
pub struct FormulaEditorVisibility(RwSignal<bool>);
impl FormulaEditorVisibility {
    pub fn new() -> Self {
        Self(RwSignal::new(false))
    }
}

#[derive(Clone)]
pub struct State {
    root_path: PathBuf,
    pub messages: RwSignal<Vec<message::Message>>,
    pub directory_tree: DirectoryTree,
    /// Active resources.
    pub selected_files: RwSignal<Vec<ResourceId>>,
    pub active_dataset: RwSignal<ActiveDataset>,
    pub datasets: Datasets,
    pub formulas: Formulas,
    pub active_formula: RwSignal<Option<ResourceId>>,
    pub canvas: Canvas,
}

impl State {
    pub fn new(root_path: PathBuf, directory_tree: lib::fs::DirectoryTree) -> Self {
        Self {
            root_path,
            messages: RwSignal::new(vec![]),
            directory_tree: DirectoryTree::from_graph(directory_tree),
            selected_files: RwSignal::new(vec![]),
            active_dataset: RwSignal::new(ActiveDataset::None),
            datasets: Datasets::new(),
            formulas: Formulas::new(),
            active_formula: RwSignal::new(None),
            canvas: Canvas::new(CANVAS_ROWS_DEFAULT, CANVAS_COLS_DEFAULT),
        }
    }

    pub fn root_path(&self) -> &PathBuf {
        &self.root_path
    }
}

#[derive(Clone, Copy, derive_more::Deref)]
pub struct Datasets(RwSignal<Vec<Dataset>>);
impl Datasets {
    pub fn new() -> Self {
        Datasets(RwSignal::new(vec![]))
    }

    /// # Returns
    /// Smallest dimensions `(rows, cols)` needed to accomodate all fixed values across workbook sheets.
    pub fn size_fixed(&self) -> (core::data::IndexType, core::data::IndexType) {
        let mut max_rows = 0;
        let mut max_cols = 0;
        for dataset in self.read_untracked().iter() {
            match dataset {
                Dataset::Csv(csv) => {
                    let (rows, cols) = csv.sheet.size_fixed();
                    if rows > max_rows {
                        max_rows = rows
                    }
                    if cols > max_cols {
                        max_cols = cols
                    }
                }

                Dataset::Workbook(workbook) => {
                    for sheet in workbook.sheets.read_untracked().iter() {
                        let (rows, cols) = sheet.size_fixed();
                        if rows > max_rows {
                            max_rows = rows
                        }
                        if cols > max_cols {
                            max_cols = cols
                        }
                    }
                }
            }
        }

        (max_rows, max_cols)
    }

    pub fn get_variable_cells_by_domain(
        &self,
        domain: &FormulaDomain,
    ) -> Vec<RwSignal<VariableCellValue>> {
        match domain {
            FormulaDomain::CsvCell { dataset, cell } => {
                let Some(dataset) = self
                    .0
                    .read_untracked()
                    .iter()
                    .find(|ds| ds.id() == dataset)
                    .cloned()
                else {
                    return vec![];
                };

                let cells = match dataset {
                    Dataset::Csv(csv) => Some(csv.sheet.cells.read_only()),

                    Dataset::Workbook(workbook) => unreachable!(),
                };
                let Some(cells) = cells else {
                    return vec![];
                };

                cells
                    .read_untracked()
                    .get(cell)
                    .map(|cell| match cell {
                        CellValue::Fixed(_) => vec![],
                        CellValue::Variable(value) => vec![value.clone()],
                    })
                    .unwrap_or(vec![])
            }

            FormulaDomain::WorkbookCell {
                dataset,
                sheet,
                cell,
            } => {
                let Some(dataset) = self
                    .0
                    .read_untracked()
                    .iter()
                    .find(|ds| ds.id() == dataset)
                    .cloned()
                else {
                    return vec![];
                };

                let cells = match dataset {
                    Dataset::Csv(csv) => unreachable!(),
                    Dataset::Workbook(workbook) => workbook
                        .sheets
                        .read_untracked()
                        .iter()
                        .find_map(|s| (s.id() == sheet).then_some(s.cells.read_only())),
                };
                let Some(cells) = cells else {
                    return vec![];
                };

                cells
                    .read_untracked()
                    .get(cell)
                    .map(|cell| match cell {
                        CellValue::Fixed(_) => vec![],
                        CellValue::Variable(value) => vec![value.clone()],
                    })
                    .unwrap_or(vec![])
            }
        }
    }
}

#[derive(Clone)]
pub enum Dataset {
    Csv(Csv),
    Workbook(Workbook),
}

impl Dataset {
    pub fn new(file: ResourceId, dataset: lib::data::Dataset) -> Self {
        match dataset {
            lib::data::Dataset::Csv(csv) => Self::Csv(Csv::new(file, csv)),
            lib::data::Dataset::Workbook(workbook) => Self::Workbook(Workbook::new(file, workbook)),
        }
    }

    pub fn id(&self) -> &ResourceId {
        match self {
            Self::Csv(csv) => csv.id(),
            Self::Workbook(workbook) => workbook.id(),
        }
    }

    pub fn is_csv(&self) -> bool {
        matches!(self, Self::Csv(_))
    }

    pub fn is_workbook(&self) -> bool {
        matches!(self, Self::Workbook(_))
    }
}

impl FileResource for Dataset {
    fn file(&self) -> &ResourceId {
        match self {
            Self::Csv(csv) => csv.file(),
            Self::Workbook(workbook) => workbook.file(),
        }
    }
}

impl core::expr::Context for &Dataset {
    fn cell_value(
        self,
        cell_ref: &hermes_core::data::CellRef,
        origin: &hermes_core::data::CellPath,
    ) -> Result<hermes_core::expr::Value, hermes_core::expr::ContextError> {
        match self {
            Dataset::Csv(csv) => csv.cell_value(cell_ref, origin),
            Dataset::Workbook(workbook) => workbook.cell_value(cell_ref, origin),
        }
    }
}

#[derive(Clone)]
pub struct Csv {
    file: ResourceId,
    inner: lib::data::Csv,
    sheet: Spreadsheet,
}

impl Csv {
    pub fn new(file: ResourceId, csv: lib::data::Csv) -> Self {
        let cells = csv.sheet.cells().clone();
        Self {
            file,
            inner: csv,
            sheet: Spreadsheet::with_fixed_values("data", cells),
        }
    }

    pub fn id(&self) -> &ResourceId {
        &self.file
    }

    pub fn sheet(&self) -> &Spreadsheet {
        &self.sheet
    }
}

impl FileResource for Csv {
    fn file(&self) -> &ResourceId {
        &self.file
    }
}

impl core::expr::Context for &Csv {
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self), level = "trace"))]
    fn cell_value(
        self,
        cell_ref: &core::data::CellRef,
        origin: &core::data::CellPath,
    ) -> Result<core::expr::Value, core::expr::ContextError> {
        let idx = core::data::CellIndex::new(cell_ref.row, cell_ref.col);
        match self
            .sheet
            .cells
            .with_untracked(|cells| cells.get(&idx).cloned())
        {
            None => {
                return Ok(core::expr::Value::Empty);
            }
            Some(cell) => match cell {
                CellValue::Fixed(data) => {
                    return Ok(data);
                }
                CellValue::Variable(data) => match data.get_untracked() {
                    VariableCellValue::Empty => return Ok(core::expr::Value::Empty),
                    VariableCellValue::Formula(data) => match data {
                        Err(err) => return Err(core::expr::ContextError::CellRefValueError(err)),
                        Ok(data) => {
                            return Ok(data);
                        }
                    },
                },
            },
        }
    }
}

#[derive(Clone)]
pub struct Workbook {
    /// Associated file id.
    file: ResourceId,
    inner: RwSignal<lib::data::Workbook>,
    pub sheets: RwSignal<Vec<Spreadsheet>>,
    pub active_sheet: RwSignal<usize>,
}

impl Workbook {
    pub fn new(file: ResourceId, workbook: lib::data::Workbook) -> Self {
        let sheets = workbook
            .sheets()
            .iter()
            .map(|(name, sheet)| Spreadsheet::with_fixed_values(name, sheet.cells().clone()))
            .collect();

        Self {
            file,
            inner: RwSignal::new(workbook),
            sheets: RwSignal::new(sheets),
            active_sheet: RwSignal::new(0),
        }
    }

    /// Alias for [`Self::file`].
    ///
    /// # Returns
    /// `ResourceId` for the workbook and associated file.
    pub fn id(&self) -> &ResourceId {
        &self.file
    }
}

impl FileResource for Workbook {
    fn file(&self) -> &ResourceId {
        &self.file
    }
}

impl core::expr::Context for &Workbook {
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self), level = "trace"))]
    fn cell_value(
        self,
        cell_ref: &core::data::CellRef,
        origin: &core::data::CellPath,
    ) -> Result<core::expr::Value, core::expr::ContextError> {
        let sheet = match cell_ref.sheet {
            core::data::SheetRef::Relative => self
                .sheets
                .read_untracked()
                .get(origin.sheet as usize)
                .cloned(),
            core::data::SheetRef::Absolute(ref sheet) => match sheet {
                core::data::SheetIndex::Label(label) => self
                    .sheets
                    .read_untracked()
                    .iter()
                    .find(|sheet| sheet.name.with_untracked(|name| name == label))
                    .cloned(),

                core::data::SheetIndex::Index(idx) => {
                    self.sheets.read_untracked().get(*idx as usize).cloned()
                }
            },
        };
        let Some(sheet) = sheet else {
            return Err(core::expr::ContextError::CellRefDoesNotExist);
        };

        let idx = core::data::CellIndex::new(cell_ref.row, cell_ref.col);
        match sheet.cells.with_untracked(|cells| cells.get(&idx).cloned()) {
            None => {
                return Ok(core::expr::Value::Empty);
            }
            Some(cell) => match cell {
                CellValue::Fixed(data) => {
                    return Ok(data);
                }
                CellValue::Variable(data) => match data.get_untracked() {
                    VariableCellValue::Empty => return Ok(core::expr::Value::Empty),
                    VariableCellValue::Formula(data) => match data {
                        Err(err) => return Err(core::expr::ContextError::CellRefValueError(err)),
                        Ok(data) => {
                            return Ok(data);
                        }
                    },
                },
            },
        }
    }
}

pub type CellMap = BTreeMap<core::data::CellIndex, CellValue>;
pub type FormulaCellValue = Result<lib::data::Data, core::expr::Error>;

#[derive(Clone)]
pub enum CellValue {
    Fixed(lib::data::Data),
    Variable(RwSignal<VariableCellValue>),
}

impl CellValue {
    pub fn fixed(value: lib::data::Data) -> Self {
        Self::Fixed(value)
    }

    pub fn empty() -> Self {
        Self::Variable(RwSignal::new(VariableCellValue::Empty))
    }

    pub fn formula(value: FormulaCellValue) -> Self {
        Self::Variable(RwSignal::new(VariableCellValue::Formula(value)))
    }
}

#[derive(Clone, derive_more::From)]
pub enum VariableCellValue {
    Empty,
    Formula(FormulaCellValue),
}

impl VariableCellValue {
    pub fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }

    pub fn unwrap(self) -> FormulaCellValue {
        match self {
            Self::Empty => panic!("called `VariableCellValue::unwrap()` on an `Empty` value"),
            Self::Formula(formula) => formula,
        }
    }
}

#[derive(Clone)]
pub struct Spreadsheet {
    id: ResourceId,
    pub name: RwSignal<String>,
    pub cells: RwSignal<CellMap>,
    /// Bounding rectangle to enclose all data, `(rows, cols)`.
    pub size: Signal<(core::data::IndexType, core::data::IndexType)>,
    /// `(rows, cols)` of fixed data.
    size_fixed: (core::data::IndexType, core::data::IndexType),
}

impl Spreadsheet {
    pub fn new(name: impl Into<String>) -> Self {
        Self::with_fixed_values(name, lib::data::CellMap::new())
    }

    pub fn with_fixed_values(name: impl Into<String>, cells: lib::data::CellMap) -> Self {
        const ROW_BUFFER: core::data::IndexType = 100;
        const COL_BUFFER: core::data::IndexType = 26;

        let size_fixed = {
            let mut max_row = 0;
            let mut max_col = 0;
            for idx in cells.keys() {
                if idx.row() > max_row {
                    max_row = idx.row() + 1;
                }
                if idx.col() > max_col {
                    max_col = idx.col() + 1;
                }
            }

            (max_row, max_col)
        };

        let mut cells = cells
            .into_iter()
            .map(|(idx, value)| (idx, CellValue::Fixed(value)))
            .collect::<CellMap>();
        let cells = RwSignal::new(cells);

        let size = Signal::derive({
            let cells = cells.read_only();
            move || {
                let mut max_row = 0;
                let mut max_col = 0;
                for idx in cells.read().keys() {
                    if idx.row() > max_row {
                        max_row = idx.row() + 1;
                    }
                    if idx.col() > max_col {
                        max_col = idx.col() + 1;
                    }
                }

                (max_row, max_col)
            }
        });

        Self {
            id: ResourceId::new(),
            name: RwSignal::new(name.into()),
            cells,
            size,
            size_fixed,
        }
    }

    pub fn id(&self) -> &ResourceId {
        &self.id
    }

    /// Bounding rectangle of fixed values, `(rows, cols)`.
    pub fn size_fixed(&self) -> (core::data::IndexType, core::data::IndexType) {
        self.size_fixed
    }
}

#[derive(Clone, Copy, derive_more::Deref)]
pub struct Formulas(RwSignal<Vec<Formula>>);
impl Formulas {
    pub fn new() -> Self {
        Self(RwSignal::new(vec![]))
    }

    pub fn get(&self, id: &ResourceId) -> Option<Formula> {
        self.read_untracked()
            .iter()
            .find(|formula| formula.id() == id)
            .cloned()
    }

    pub fn get_by_containing_domain(&self, domain: &FormulaDomain) -> Option<Formula> {
        self.read_untracked()
            .iter()
            .find(|formula| {
                formula
                    .domain
                    .with_untracked(|f_domain| f_domain.contains(domain))
            })
            .cloned()
    }
}

#[derive(Clone)]
pub struct Formula {
    id: ResourceId,
    pub domain: RwSignal<FormulaDomain>,
    pub value: RwSignal<String>,
}

impl Formula {
    pub fn new(domain: FormulaDomain) -> Self {
        Self {
            id: ResourceId::new(),
            domain: RwSignal::new(domain),
            value: RwSignal::new("".to_string()),
        }
    }

    pub fn id(&self) -> &ResourceId {
        &self.id
    }
}

#[derive(Clone, PartialEq)]
pub enum FormulaDomain {
    /// A single cell in a csv.
    CsvCell {
        dataset: ResourceId,
        cell: core::data::CellIndex,
    },

    /// A single cell in a workbook.
    WorkbookCell {
        dataset: ResourceId,
        sheet: ResourceId,
        cell: core::data::CellIndex,
    },
}

impl FormulaDomain {
    /// Test if the domain intersects with the given domain.
    pub fn intersects(&self, domain: &Self) -> bool {
        match (self, domain) {
            (Self::CsvCell { .. }, Self::CsvCell { .. }) => self == domain,
            (Self::WorkbookCell { .. }, Self::WorkbookCell { .. }) => self == domain,
            (Self::CsvCell { .. }, Self::WorkbookCell { .. }) => false,
            (Self::WorkbookCell { .. }, Self::CsvCell { .. }) => false,
        }
    }

    /// Test if the domain fully contains the given domain.
    pub fn contains(&self, domain: &Self) -> bool {
        match (self, domain) {
            (Self::CsvCell { .. }, Self::CsvCell { .. }) => self == domain,
            (Self::WorkbookCell { .. }, Self::WorkbookCell { .. }) => self == domain,
            (Self::CsvCell { .. }, Self::WorkbookCell { .. }) => false,
            (Self::WorkbookCell { .. }, Self::CsvCell { .. }) => false,
        }
    }
}

#[derive(Clone)]
pub struct File {
    id: ResourceId,
    pub name: RwSignal<OsString>,
}

impl File {
    pub fn id(&self) -> &ResourceId {
        &self.id
    }
}

impl From<OsString> for File {
    fn from(value: OsString) -> Self {
        Self {
            id: ResourceId::new(),
            name: RwSignal::new(value),
        }
    }
}

/// Maintains a list of files sorted by name.
#[derive(Clone, Debug, derive_more::Deref)]
pub struct FileList {
    #[deref]
    files: RwSignal<Vec<File>>,
    _sort_guard: Effect<LocalStorage>,
}

impl FileList {
    pub fn with_files(files: Vec<File>) -> Self {
        let files = RwSignal::new(files);
        let _sort_guard = Effect::watch(
            move || {
                files.read().iter().for_each(|file| file.name.track());
            },
            move |_, _, _| {
                let mut files = files.get_untracked();
                files.sort_by_key(|file| file.name.get_untracked());
                files
            },
            true,
        );

        Self { files, _sort_guard }
    }
}

#[derive(Clone)]
pub struct Directory {
    id: ResourceId,
    pub name: RwSignal<OsString>,
    pub files: FileList,
}

impl Directory {
    pub fn id(&self) -> &ResourceId {
        &self.id
    }
}

impl From<lib::fs::Directory> for Directory {
    fn from(value: lib::fs::Directory) -> Self {
        let lib::fs::Directory { name, files } = value;
        let files = files.into_iter().map(|file| file.into()).collect();

        Self {
            id: ResourceId::new(),
            name: RwSignal::new(name),
            files: FileList::with_files(files),
        }
    }
}

#[derive(Clone)]
pub struct DirectoryTree {
    directories: RwSignal<Vec<Directory>>,
    parents: RwSignal<Vec<usize>>,
}

impl DirectoryTree {
    pub const ROOT: usize = 0;

    pub fn from_graph(graph: lib::fs::DirectoryTree) -> Self {
        let directories = graph
            .directories()
            .iter()
            .map(|dir| dir.clone().into())
            .collect();

        Self {
            directories: RwSignal::new(directories),
            parents: RwSignal::new(graph.parents().clone()),
        }
    }

    pub fn root(&self) -> Directory {
        self.directories
            .with_untracked(|dirs| dirs[Self::ROOT].clone())
    }

    /// Get the current index of the directory.
    ///
    /// # Notes
    /// + Indexes are not stable across write operations.
    fn index(&self, directory: &ResourceId) -> Option<usize> {
        self.directories
            .read_untracked()
            .iter()
            .position(|dir| dir.id() == directory)
    }

    /// Create a `leptos::Signal` tracking the index of the directory.
    fn index_tracked(&self, directory: ResourceId) -> Signal<Option<usize>> {
        Signal::derive({
            let directories = self.directories.read_only();
            move || {
                directories
                    .read()
                    .iter()
                    .position(|dir| *dir.id() == directory)
            }
        })
    }

    /// Get a directory by it's index.
    ///
    /// # Notes
    /// + Indexes are not stable across write operations.
    fn get_idx(&self, directory: usize) -> Result<Directory, lib::fs::error::NodeDoesNotExist> {
        self.directories
            .read_untracked()
            .get(directory)
            .map(|dir| dir.clone())
            .ok_or(lib::fs::error::NodeDoesNotExist)
    }

    pub fn get_file_by_id(&self, id: &ResourceId) -> Option<File> {
        self.directories
            .read_untracked()
            .iter()
            .find_map(|directory| {
                directory
                    .files
                    .read_untracked()
                    .iter()
                    .find(|file| file.id() == id)
                    .cloned()
            })
    }

    /// Gets the current path to the file relative to the directory tree root.
    pub fn get_file_path(&self, id: &ResourceId) -> Option<PathBuf> {
        let (parent_idx, filename) =
            self.directories
                .read_untracked()
                .iter()
                .enumerate()
                .find_map(|(idx, directory)| {
                    directory.files.read_untracked().iter().find_map(|file| {
                        (file.id() == id).then_some((idx, file.name.get_untracked()))
                    })
                })?;

        let ancestors = self.ancestors_idx(parent_idx).ok()?;
        let path = self.directories.with_untracked(move |directories| {
            ancestors
                .into_iter()
                .rev()
                .skip(1)
                .map(|idx| directories[idx].name.get_untracked())
                .collect::<PathBuf>()
        });

        Some(path.join(filename))
    }

    /// # Returns
    /// List of ancestors starting with `child` and ending with the graph root.
    fn ancestors_idx(&self, child: usize) -> Result<Vec<usize>, lib::fs::error::NodeDoesNotExist> {
        if child > self.directories.read_untracked().len() {
            return Err(lib::fs::error::NodeDoesNotExist);
        }

        let ancestors = self.parents.with_untracked(|parents| {
            let mut ancestors = vec![child];
            let mut child = child;
            while child != Self::ROOT {
                let parent = parents[child - 1];
                ancestors.push(parent);
                child = parent
            }

            ancestors
        });

        Ok(ancestors)
    }

    /// Get the parent index.
    ///
    /// # Notes
    /// + Indexes are not stable across write operations.
    fn parent_idx(&self, child: usize) -> Result<Option<usize>, lib::fs::error::NodeDoesNotExist> {
        if child >= self.directories.read_untracked().len() {
            return Err(lib::fs::error::NodeDoesNotExist);
        }
        if child == Self::ROOT {
            return Ok(None);
        }

        Ok(Some(
            self.parents.with_untracked(|parents| parents[child - 1]),
        ))
    }

    /// Get children indexes.
    ///
    /// # Notes
    /// + Indexes are not stable across write operations.
    fn children_idx(&self, parent: usize) -> Result<Vec<usize>, lib::fs::error::NodeDoesNotExist> {
        if parent >= self.directories.read_untracked().len() {
            return Err(lib::fs::error::NodeDoesNotExist);
        }

        Ok(self
            .parents
            .read_untracked()
            .iter()
            .enumerate()
            .filter_map(|(child, c_parent)| (*c_parent == parent).then_some(child))
            .collect())
    }

    pub fn children(
        &self,
        parent: ResourceId,
    ) -> Signal<Result<Vec<Directory>, lib::fs::error::NodeDoesNotExist>> {
        let parent_idx = self.index_tracked(parent);
        Signal::derive({
            let directories = self.directories.read_only();
            let parents = self.parents.read_only();
            move || {
                let parent = parent_idx.read().ok_or(lib::fs::error::NodeDoesNotExist)?;
                if parent >= directories.read().len() {
                    return Err(lib::fs::error::NodeDoesNotExist);
                }

                let children_idx = parents
                    .read()
                    .iter()
                    .enumerate()
                    .filter_map(|(child, c_parent)| (*c_parent == parent).then_some(child))
                    .collect::<Vec<_>>();

                let children = directories.with(|directories| {
                    let mut children = Vec::with_capacity(children_idx.len());
                    for child in children_idx {
                        children.push(directories[child + 1].clone());
                    }
                    children
                });

                Ok(children)
            }
        })
    }
}

#[derive(Clone)]
pub struct Canvas {
    cells: CanvasCells,
    rows: RwSignal<core::data::IndexType>,
    cols: RwSignal<core::data::IndexType>,
}
impl Canvas {
    pub fn new(rows: core::data::IndexType, cols: core::data::IndexType) -> Self {
        Self {
            cells: CanvasCells::new(rows, cols),
            rows: RwSignal::new(rows),
            cols: RwSignal::new(cols),
        }
    }

    pub fn cells(&self) -> CanvasCells {
        self.cells
    }

    pub fn rows(&self) -> ReadSignal<core::data::IndexType> {
        self.rows.read_only()
    }

    pub fn cols(&self) -> ReadSignal<core::data::IndexType> {
        self.cols.read_only()
    }
}

#[derive(Clone)]
pub enum CanvasCellValue {
    Unset,
    Set(CellValue),
}
impl CanvasCellValue {
    pub fn unwrap(&self) -> &CellValue {
        match self {
            Self::Unset => panic!("tried to unwrap an unset canvas cell value"),
            Self::Set(value) => value,
        }
    }

    pub fn is_set(&self) -> bool {
        matches!(self, Self::Set(_))
    }

    pub fn is_empty(&self) -> bool {
        let Self::Set(CellValue::Variable(value)) = self else {
            return false;
        };
        value.read_untracked().is_empty()
    }

    pub fn insert(&mut self, value: CellValue) {
        *self = Self::Set(value);
    }

    pub fn take(&mut self) {
        *self = Self::Unset
    }
}

#[derive(Clone, Copy)]
pub struct CanvasCells {
    inner: RwSignal<Vec<Vec<RwSignal<CanvasCellValue>>>>,
}
impl CanvasCells {
    pub fn new(rows: core::data::IndexType, cols: core::data::IndexType) -> Self {
        let mut cells = Vec::with_capacity(rows as usize);
        for _ in 0..rows {
            let mut row = Vec::with_capacity(cols as usize);
            for _ in 0..cols {
                row.push(RwSignal::new(CanvasCellValue::Unset));
            }
            cells.push(row);
        }

        Self {
            inner: RwSignal::new(cells),
        }
    }

    pub fn get_cell(&self, idx: &core::data::CellIndex) -> Option<RwSignal<CanvasCellValue>> {
        let row = idx.row() as usize;
        let col = idx.col() as usize;
        let rows = self.inner.read_untracked().len();
        if row >= rows {
            return None;
        }
        let cols = self.inner.read_untracked()[0].len();
        if col >= cols {
            return None;
        }
        Some(self.inner.read_untracked()[row][col].clone())
    }

    /// Unset all cells.
    pub fn clear(&self) {
        self.inner.with_untracked(|cells| {
            for row in cells.iter() {
                for cell in row.iter() {
                    if cell.read_untracked().is_set() {
                        cell.update(|cell| cell.take());
                    }
                }
            }
        });
    }

    /// Set all cells to empty.
    pub fn empty(&self) {
        self.inner.with_untracked(|cells| {
            for row in cells.iter() {
                for cell in row.iter() {
                    if !cell.read_untracked().is_empty() {
                        cell.update(|cell| cell.insert(CellValue::empty()));
                    }
                }
            }
        });
    }
}
