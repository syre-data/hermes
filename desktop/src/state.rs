use crate::{formula, message};
use hermes_core as core;
use hermes_desktop_lib as lib;
use leptos::prelude::*;
use std::{collections::BTreeMap, ffi::OsString, path::PathBuf, sync::Arc};

#[derive(Clone, derive_more::Deref, Hash, PartialEq, Eq, Debug)]
pub struct ResourceId(uuid::Uuid);
impl ResourceId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

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

#[derive(Clone, derive_more::Deref)]
pub struct WorkspaceOwner(Owner);
impl WorkspaceOwner {
    pub fn with_current() -> Self {
        Self(Owner::current().expect("owner to exist"))
    }
}

#[derive(Clone)]
pub enum ActiveWorkbook {
    None,
    Some {
        id: ResourceId,
        active_cell: RwSignal<ActiveCell>,
    },
}

impl ActiveWorkbook {
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
    pub active_workbook: RwSignal<ActiveWorkbook>,
    pub workbooks: Workbooks,
    pub formulas: Formulas,
    pub active_formula: RwSignal<Option<ResourceId>>,
}

impl State {
    pub fn new(root_path: PathBuf, directory_tree: lib::fs::DirectoryTree) -> Self {
        Self {
            root_path,
            messages: RwSignal::new(vec![]),
            directory_tree: DirectoryTree::from_graph(directory_tree),
            selected_files: RwSignal::new(vec![]),
            active_workbook: RwSignal::new(ActiveWorkbook::None),
            workbooks: Workbooks::new(),
            formulas: Formulas::new(),
            active_formula: RwSignal::new(None),
        }
    }

    pub fn root_path(&self) -> &PathBuf {
        &self.root_path
    }
}

#[derive(Clone, Copy, derive_more::Deref)]
pub struct Workbooks(RwSignal<Vec<Workbook>>);
impl Workbooks {
    pub fn new() -> Self {
        Workbooks(RwSignal::new(vec![]))
    }

    /// # Returns
    /// Smallest dimensions `(rows, cols)` needed to accomodate all workbook sheets.
    pub fn size(&self) -> (core::data::IndexType, core::data::IndexType) {
        let mut max_rows = 0;
        let mut max_cols = 0;
        for workbook in self.read_untracked().iter() {
            for sheet in workbook.sheets.read_untracked().iter() {
                let (rows, cols) = sheet.size.get_untracked();
                if rows > max_rows {
                    max_rows = rows
                }
                if cols > max_cols {
                    max_cols = cols
                }
            }
        }

        (max_rows, max_cols)
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
            .map(|(name, sheet)| Spreadsheet::with_cells(name, sheet.cells().clone()))
            .collect();
        Self {
            file,
            inner: RwSignal::new(workbook),
            sheets: RwSignal::new(sheets),
            active_sheet: RwSignal::new(0),
        }
    }

    pub fn file(&self) -> &ResourceId {
        &self.file
    }

    /// Alias for [`Self::file`].
    ///
    /// # Returns
    /// `ResourceId` for the workbook and associated file.
    pub fn id(&self) -> &ResourceId {
        &self.file
    }

    pub fn kind(&self) -> lib::data::WorkbookKind {
        self.inner.with_untracked(|wb| wb.kind())
    }

    pub fn is_csv(&self) -> bool {
        self.inner.with_untracked(|wb| {
            let kind = wb.kind();
            matches!(lib::data::WorkbookKind::Csv, kind)
        })
    }
}

pub type CellMap = BTreeMap<core::data::CellIndex, lib::data::Data>;

#[derive(Clone)]
pub struct Spreadsheet {
    id: ResourceId,
    pub name: RwSignal<String>,
    pub cells: RwSignal<CellMap>,
    /// `(rows, cols)` of data.
    /// Excludes applied formulas.
    pub size: Signal<(core::data::IndexType, core::data::IndexType)>,
}

impl Spreadsheet {
    pub fn new(name: impl Into<String>) -> Self {
        Self::with_cells(name, CellMap::new())
    }

    pub fn with_cells(name: impl Into<String>, cells: CellMap) -> Self {
        let cells = RwSignal::new(cells);
        let size = Signal::derive({
            let cells = cells.read_only();
            move || {
                let mut max_row = 0;
                let mut max_col = 0;
                for idx in cells.read().keys() {
                    if idx.row() > max_row {
                        max_row = idx.row();
                    }
                    if idx.col() > max_col {
                        max_col = idx.col();
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
        }
    }

    pub fn id(&self) -> &ResourceId {
        &self.id
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
    /// A single cell.
    Cell {
        workbook: ResourceId,
        sheet: ResourceId,
        cell: core::data::CellIndex,
    },
}

impl FormulaDomain {
    /// Test if the domain intersects with the given domain.
    pub fn intersects(&self, domain: &Self) -> bool {
        match (self, domain) {
            (Self::Cell { .. }, Self::Cell { .. }) => self == domain,
        }
    }

    /// Test if the domain fully contains the given domain.
    pub fn contains(&self, domain: &Self) -> bool {
        match (self, domain) {
            (Self::Cell { .. }, Self::Cell { .. }) => self == domain,
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
