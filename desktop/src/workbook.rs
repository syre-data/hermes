use crate::{formula, icon, state, types};
use hermes_core as core;
use hermes_desktop_lib as lib;
use leptos::{
    either::{Either, EitherOf3},
    ev,
    prelude::*,
};
use leptos_icons::Icon;
use std::{collections::btree_map::Values, path::PathBuf};

#[component]
pub fn Workspace() -> impl IntoView {
    let state = expect_context::<state::State>();

    let active = state.active_workbook.read_only();
    let workbooks = state.workbooks.read_only();
    move || {
        active.with(|active| {
            if let state::ActiveWorkbook::Some { id, .. } = active {
                let workbook = workbooks
                    .read_untracked()
                    .iter()
                    .find(|workbook| workbook.file() == id)
                    .expect("workbook to exist")
                    .clone();
                Either::Left(view! { <Workbook workbook /> })
            } else {
                Either::Right(view! { <NoActiveFile /> })
            }
        })
    }
}

#[component]
fn NoActiveFile() -> impl IntoView {
    view! { <div class="p-2 text-center">"Select a file"</div> }
}

#[derive(Clone, derive_more::Deref)]
struct ActiveWorkbookId(state::ResourceId);

#[component]
fn Workbook(workbook: state::Workbook) -> impl IntoView {
    let formula_editor_vis = expect_context::<state::FormulaEditorVisibility>();
    provide_context(ActiveWorkbookId(workbook.id().clone()));

    let workbook = workbook.clone();
    move || {
        let sheet_names = workbook
            .sheets
            .read()
            .iter()
            .map(|sheet| sheet.name.get())
            .collect::<Vec<_>>();

        match workbook.kind() {
            lib::data::WorkbookKind::Csv => {
                let sheet = workbook.sheets.read().get(0).expect("sheet exists").clone();
                Either::Left(view! {
                    <div class="flex flex-col h-full min-h-0">
                        <Spreadsheet sheet class="grow" />
                        <FormulaEditor />
                    </div>
                })
            }
            lib::data::WorkbookKind::Workbook => Either::Right(view! {
                <div class="flex flex-col h-full w-full">
                    <div class="grow">
                        {
                            let active_sheet = workbook.active_sheet.read_only();
                            let workbook = workbook.clone();
                            move || {
                                let sheet = workbook
                                    .sheets
                                    .read()
                                    .get(active_sheet.get())
                                    .expect("sheet exists")
                                    .clone();
                                view! { <Spreadsheet sheet class="grow" /> }
                            }
                        }
                    </div>
                    <div>
                        <SheetList sheets=sheet_names />
                    </div>
                    <FormulaEditor />
                </div>
            }),
        }
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

#[derive(Clone, derive_more::Deref)]
struct ActiveSpreadsheet(state::ResourceId);

#[component]
fn Spreadsheet(
    sheet: state::Spreadsheet,
    #[prop(optional, into)] class: Option<String>,
) -> impl IntoView {
    const ROW_BUFFER: core::data::IndexType = 100;
    const COL_BUFFER: core::data::IndexType = 26;
    const WRAPPER_CLASS: &'static str = "overflow-auto scrollbar-thin";

    let state = expect_context::<state::State>();
    let active_wb = expect_context::<ActiveWorkbookId>();

    provide_context(ActiveSpreadsheet(sheet.id().clone()));

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
                            let size = sheet.size;
                            move || {
                                let num_cols = size.get().1 + COL_BUFFER;
                                (0..num_cols)
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
                        let formulas = state.formulas;
                        let workbook_id = (*active_wb).clone();
                        let sheet_id = sheet.id().clone();
                        let size = sheet.size;
                        move || {
                            let rows = move || {
                                let num_rows = size.get().0 + ROW_BUFFER;
                                0..num_rows
                            };
                            let cols = move || {
                                let num_cols = size.get().1 + COL_BUFFER;
                                0..num_cols
                            };
                            view! {
                                <For each=rows key=|row| *row let:row_idx>
                                    <tr>
                                        <th class="sticky left-0 cursor-pointer bg-white dark:bg-secondary-800">
                                            {core::utils::index_to_row(row_idx)}
                                        </th>
                                        <For each=cols key=|col| *col let:col_idx>
                                            {
                                                let idx: core::data::CellIndex = (row_idx, col_idx).into();
                                                move || match sheet.cells.read().get(&idx) {
                                                    None => {
                                                        EitherOf3::A(view! { <CellEmpty idx=idx.clone() /> })
                                                    }
                                                    Some(data) => {
                                                        match data {
                                                            state::CellValue::Fixed(data) => {
                                                                EitherOf3::B(
                                                                    view! {
                                                                        <CellValueFixed data=data.clone() idx=idx.clone() />
                                                                    },
                                                                )
                                                            }
                                                            state::CellValue::Variable(data) => {
                                                                EitherOf3::C(
                                                                    view! {
                                                                        <CellValueVariable data=data.read_only() idx=idx.clone() />
                                                                    },
                                                                )
                                                            }
                                                        }
                                                    }
                                                }
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

const STATIC_CELL_DATA_CLASS: &'static str =
    "cursor-pointer hover:bg-secondary-50 dark:hover:bg-secondary-700";

/// Cell data for static data.
#[component]
fn CellValueFixed(data: lib::data::Data, idx: core::data::CellIndex) -> impl IntoView {
    view! {
        <td class=STATIC_CELL_DATA_CLASS data-row=idx.row() data-col=idx.col()>
            {calamine_data_to_string(&data)}
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
                Ok(data) => calamine_data_to_string(data),
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
    let active_workbook = expect_context::<ActiveWorkbookId>();
    let active_sheet = expect_context::<ActiveSpreadsheet>();
    let formula_editor_vis = expect_context::<state::FormulaEditorVisibility>();

    let create_cell_data = {
        let formulas = state.formulas;
        let active_formula = state.active_formula;
        let idx = idx.clone();
        move |e: ev::MouseEvent| {
            if e.button() != types::MouseButton::Primary {
                return;
            }

            let domain = state::FormulaDomain::Cell {
                workbook: (*active_workbook).clone(),
                sheet: (*active_sheet).clone(),
                cell: idx.clone(),
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

fn calamine_data_to_string(data: &lib::data::Data) -> String {
    use lib::data::Data;

    match data {
        Data::String(val) => val.clone(),
        Data::Float(val) => val.to_string(),
        Data::Int(val) => val.to_string(),
        Data::Bool(val) => val.to_string(),
        Data::Empty => "".to_string(),
        Data::DateTime(val) => val.to_string(),
        Data::DateTimeIso(val) => val.clone(),
        Data::DurationIso(val) => val.clone(),
        Data::Error(val) => val.to_string(),
    }
}
