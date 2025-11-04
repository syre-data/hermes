use crate::{formula, icon, state, state::FileResource, types};
use hermes_core as core;
use hermes_desktop_lib as lib;
use leptos::{
    either::{Either, EitherOf3, either},
    ev,
    prelude::*,
};
use leptos_icons::Icon;
use std::{collections::btree_map::Values, path::PathBuf};

#[derive(Copy, Clone, derive_more::Deref)]
struct ActiveDatasetId(Signal<Option<state::ResourceId>>);
impl ActiveDatasetId {
    pub fn from_active_dataset(base: ReadSignal<state::ActiveDataset>) -> Self {
        Self(Signal::derive(move || base.read().as_ref().cloned()))
    }
}

#[derive(Copy, Clone, derive_more::Deref)]
struct ActiveSpreadsheetId(RwSignal<Option<state::ResourceId>>);
impl ActiveSpreadsheetId {
    pub fn new() -> Self {
        Self(RwSignal::new(None))
    }
}

#[component]
pub fn Workspace() -> impl IntoView {
    let state = expect_context::<state::State>();
    provide_context(ActiveDatasetId::from_active_dataset(
        state.active_dataset.read_only(),
    ));
    provide_context(ActiveSpreadsheetId::new());

    let active = state.active_dataset.read_only();
    let datasets = state.datasets.read_only();
    let mut canvas = state.canvas.clone();
    view! {
        <div class="h-full flex flex-col">
            <NoActiveFile {..} class:hidden=move || active.read().is_some() />
            <Canvas class="grow" class:hidden=move || active.read().is_none() />
            {move || {
                active
                    .with(|active| {
                        if let state::ActiveDataset::Some { id, .. } = active {
                            let dataset = datasets
                                .read_untracked()
                                .iter()
                                .find(|dataset| dataset.file() == id)
                                .expect("workbook to exist")
                                .clone();
                            Some(
                                either!(
                                    dataset,
                                    state::Dataset::Csv(csv) => view! { <Csv csv /> },
                                    state::Dataset::Workbook(workbook) => view! { <Workbook workbook /> },
                                ),
                            )
                        } else {
                            canvas.cells().clear();
                            None
                        }
                    })
            }}
        </div>
    }
}

#[component]
fn NoActiveFile() -> impl IntoView {
    view! { <div class="p-2 text-center">"Select a file"</div> }
}

#[component]
fn Canvas(#[prop(optional, into)] class: Option<String>) -> impl IntoView {
    const WRAPPER_CLASS: &'static str = "overflow-auto scrollbar-thin";

    let state = expect_context::<state::State>();
    let canvas = state.canvas;

    let wrapper_class = if let Some(class) = class {
        format!("{class} {WRAPPER_CLASS}")
    } else {
        WRAPPER_CLASS.to_string()
    };

    view! {
        <div class=wrapper_class>
            <table class="table-fixed">
                <thead class="bg-white dark:bg-secondary-800 sticky top-0">
                    <tr>
                        <th></th>
                        {
                            let cols = canvas.cols();
                            move || {
                                (0..cols.get())
                                    .into_iter()
                                    .map(|idx| {
                                        view! {
                                            <th class="cursor-pointer">
                                                {core::utils::index_to_col(idx)}
                                            </th>
                                        }
                                    })
                                    .collect::<Vec<_>>()
                            }
                        }
                    </tr>
                </thead>
                <tbody>
                    {
                        let cells = canvas.cells();
                        let rows = canvas.rows();
                        let cols = canvas.cols();
                        move || {
                            view! {
                                <For each=move || 0..rows.get() key=|row| *row let:row_idx>
                                    <tr>
                                        <th class="sticky left-0 cursor-pointer bg-white dark:bg-secondary-800">
                                            {core::utils::index_to_row(row_idx)}
                                        </th>
                                        <For each=move || 0..cols.get() key=|col| *col let:col_idx>
                                            {
                                                let idx: core::data::CellIndex = (row_idx, col_idx).into();
                                                let cell = cells
                                                    .get_cell(&idx)
                                                    .expect("cell to exist")
                                                    .read_only();
                                                view! { <CanvasCellValue idx cell /> }
                                            }
                                        </For>
                                    </tr>
                                </For>
                            }
                        }
                    }
                </tbody>
            </table>
        </div>
    }
}

#[component]
fn CanvasCellValue(
    idx: core::data::CellIndex,
    cell: ReadSignal<state::CanvasCellValue>,
) -> impl IntoView {
    move || match cell.get() {
        state::CanvasCellValue::Unset => EitherOf3::A(view! { <CellValueUnset /> }),
        state::CanvasCellValue::Set(value) => match value {
            state::CellValue::Fixed(data) => {
                EitherOf3::B(view! { <CellValueFixed data=data.clone() idx=idx.clone() /> })
            }
            state::CellValue::Variable(data) => {
                EitherOf3::C(view! { <CellValueVariable data=data.read_only() idx=idx.clone() /> })
            }
        },
    }
}

#[component]
fn CellValueUnset() -> impl IntoView {
    view! { <td class="cursor-not-allowed"></td> }
}

const STATIC_CELL_DATA_CLASS: &'static str =
    "cursor-pointer hover:bg-secondary-50 dark:hover:bg-secondary-700";

/// Cell data for static data.
#[component]
fn CellValueFixed(data: lib::data::Data, idx: core::data::CellIndex) -> impl IntoView {
    view! {
        <td class=STATIC_CELL_DATA_CLASS data-row=idx.row() data-col=idx.col()>
            // {calamine_data_to_string(&data)}
            {data.to_string()}
        </td>
    }
}

#[component]
fn CellValueVariable(
    data: ReadSignal<state::VariableCellValue>,
    idx: core::data::CellIndex,
) -> impl IntoView {
    move || match data.get() {
        state::VariableCellValue::Empty => Either::Left(view! { <CellEmpty idx=idx.clone() /> }),
        state::VariableCellValue::Formula(data) => {
            Either::Right(view! { <CellValueFormula data idx=idx.clone() /> })
        }
    }
}

/// Cell data for dynamic data with a formula.
#[component]
fn CellValueFormula(
    data: Result<lib::data::Data, core::expr::Error>,
    idx: core::data::CellIndex,
) -> impl IntoView {
    let state = expect_context::<state::State>();

    let select_formula = move |e: ev::MouseEvent| {
        if e.button() != types::MouseButton::Primary {
            return;
        }
    };

    view! {
        <td
            class="cursor-pointer hover:bg-secondary-50 dark:hover:bg-secondary-700 border border-primary-600"
            class:bg-brand-red-500=data.is_err()
            data-row=idx.row()
            data-col=idx.col()
            on:mousedown=select_formula
        >
            {match data.as_ref() {
                Ok(data) => data.to_string(),
                Err(err) => todo!(),
            }}

        </td>
    }
}

/// Cell data for an empty cell.
#[component]
fn CellEmpty(idx: core::data::CellIndex) -> impl IntoView {
    let state = expect_context::<state::State>();
    let workspace_owner = expect_context::<state::WorkspaceOwner>();
    let active_dataset = expect_context::<ActiveDatasetId>();
    let active_sheet = expect_context::<ActiveSpreadsheetId>();
    let formula_editor_vis = expect_context::<state::FormulaEditorVisibility>();

    let create_cell_data = {
        let datasets = state.datasets;
        let formulas = state.formulas;
        let active_formula = state.active_formula;
        let idx = idx.clone();
        move |e: ev::MouseEvent| {
            if e.button() != types::MouseButton::Primary {
                return;
            }

            let dataset = active_dataset.with_untracked(|active_dataset| {
                let active_dataset = active_dataset.as_ref().unwrap();
                datasets
                    .read_untracked()
                    .iter()
                    .find(|ds| ds.id() == active_dataset)
                    .expect("dataset to exist")
                    .clone()
            });
            let domain = match dataset {
                state::Dataset::Csv(csv) => state::FormulaDomain::CsvCell {
                    dataset: active_dataset
                        .get_untracked()
                        .expect("dataset id to be set"),
                    cell: idx.clone(),
                },
                state::Dataset::Workbook(workbook) => state::FormulaDomain::WorkbookCell {
                    dataset: active_dataset
                        .get_untracked()
                        .expect("dataset id to be set"),
                    sheet: active_sheet
                        .get_untracked()
                        .expect("spreadsheet id to be set"),
                    cell: idx.clone(),
                },
            };

            let formula_id = if let Some(formula) = formulas.get_by_containing_domain(&domain) {
                formula.id().clone()
            } else {
                let formula = workspace_owner.with(|| state::Formula::new(domain));
                let id = formula.id().clone();
                formulas.write().push(formula);
                id
            };

            let _ = active_formula.write().insert(formula_id);
            formula_editor_vis.set(true);
        }
    };

    view! {
        <td
            class=STATIC_CELL_DATA_CLASS
            on:click=create_cell_data
            data-row=idx.row()
            data-col=idx.col()
        ></td>
    }
}

#[component]
fn Csv(csv: state::Csv) -> impl IntoView {
    view! {
        <Spreadsheet sheet=csv.sheet().clone() />
        <FormulaEditor />
    }
}

#[component]
fn Workbook(workbook: state::Workbook) -> impl IntoView {
    let active_sheet = expect_context::<ActiveSpreadsheetId>();
    let formula_editor_vis = expect_context::<state::FormulaEditorVisibility>();

    let sheet_names = workbook
        .sheets
        .read()
        .iter()
        .map(|sheet| sheet.name.get())
        .collect::<Vec<_>>();

    let active_sheet = workbook.active_sheet.read_only();
    let workbook = workbook.clone();
    let sheet = workbook
        .sheets
        .read()
        .get(active_sheet.get())
        .expect("sheet exists")
        .clone();

    view! {
        <Spreadsheet sheet />
        <div>
            <SheetList sheets=sheet_names />
        </div>
        <FormulaEditor />
    }
}

#[component]
fn FormulaEditor() -> impl IntoView {
    let state = expect_context::<state::State>();
    let formula_editor_vis = expect_context::<state::FormulaEditorVisibility>();
    let active_formula = state.active_formula.read_only();

    let close_formula_editor = move |e: ev::MouseEvent| {
        if e.button() != types::MouseButton::Primary {
            return;
        }

        formula_editor_vis.set(false);
    };

    Effect::watch(
        active_formula,
        {
            let formulas = state.formulas;
            move |_, prev, _| {
                if let Some(Some(prev)) = prev {
                    if let Some(formula) = formulas.get(prev) {
                        if formula.value.read_untracked().trim().is_empty() {
                            formulas.update(|formulas| formulas.retain(|f| f.id() != formula.id()))
                        }
                    }
                }
            }
        },
        false,
    );

    view! {
        <div
            class="flex bg-white dark:bg-secondary-800"
            class:hidden=move || !formula_editor_vis.get()
        >
            <formula::Editor />
            <div>
                <button type="button" class="cursor-pointer" on:mousedown=close_formula_editor>
                    <Icon icon=icon::Close />
                </button>
            </div>
        </div>
    }
}

#[component]
fn Spreadsheet(sheet: state::Spreadsheet) -> impl IntoView {
    let state = expect_context::<state::State>();
    let owner = expect_context::<state::WorkspaceOwner>();

    let canvas = state.canvas.cells();
    owner.with(|| canvas.empty());
    let size = sheet.size;
    move || {
        for row_idx in 0..size.get().0 {
            for col_idx in 0..size.get().1 {
                let idx: core::data::CellIndex = (row_idx, col_idx).into();
                let cell = canvas.get_cell(&idx).expect("canvas cell to exist");
                if let Some(data) = sheet.cells.read().get(&idx) {
                    cell.update(|cell| cell.insert(data.clone()))
                }
            }
        }
    }
}

#[component]
fn SheetList(sheets: Vec<String>) -> impl IntoView {
    view! {
        <div class="flex">
            {sheets
                .into_iter()
                .map(|name| view! { <div class="pl-2 pr-8">{name}</div> })
                .collect::<Vec<_>>()}
        </div>
    }
}

fn expr_value_to_string(value: &core::expr::Value) -> String {
    match value {
        core::expr::Value::Empty => "".to_string(),
        core::expr::Value::String(value) => value.clone(),
        core::expr::Value::Int(value) => value.to_string(),
        core::expr::Value::Float(value) => value.to_string(),
        core::expr::Value::Bool(value) => value.to_string(),
        core::expr::Value::DateTime(date_time) => todo!(),
        core::expr::Value::Duration(duration) => todo!(),
    }
}

fn expr_error_to_string(error: &core::expr::Error) -> String {
    match error {
        core::expr::Error::Tokenize(kind) => todo!(),
        core::expr::Error::Parse(kind) => todo!(),
        core::expr::Error::Div0 => "#Div0".to_string(),
        core::expr::Error::InvalidNumber => "#NaN".to_string(),
        core::expr::Error::InvalidOperation(_) => "#BadOp".to_string(),
        core::expr::Error::Overflow => "#Overflow".to_string(),
        core::expr::Error::InvalidCellRef(cell_ref) => "#CellRef".to_string(),
    }
}
