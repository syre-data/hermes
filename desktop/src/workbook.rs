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
                Either::Right(view! {
                    <div class="relative h-full">
                        <Spreadsheet sheet />
                        <FormulaEditor />
                    </div>
                })
            }
            lib::data::WorkbookKind::Workbook => Either::Left(view! {
                <div class="relative flex flex-col h-full w-full">
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
                                view! { <Spreadsheet sheet /> }
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
            class="flex absolute bottom-0 left-0 right-0 bg-white dark:bg-secondary-800"
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
fn Spreadsheet(sheet: state::Spreadsheet) -> impl IntoView {
    let state = expect_context::<state::State>();
    let active_wb = expect_context::<ActiveWorkbookId>();

    pub const ROW_BUFFER: usize = 100;
    pub const COL_BUFFER: usize = 26;
    provide_context(ActiveSpreadsheet(sheet.id().clone()));

    view! {
        <div class="h-full overflow-auto scrollbar-thin">
            <table>
                <thead class="bg-white dark:bg-secondary-800 sticky top-0">
                    <tr>
                        <th></th>
                        {
                            let size = sheet.size;
                            move || {
                                let num_cols = size.get().1 + COL_BUFFER as core::data::IndexType;
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
                            let (num_rows, num_cols) = size.get();
                            let num_rows = num_rows + ROW_BUFFER as core::data::IndexType;
                            let num_cols = num_cols + COL_BUFFER as core::data::IndexType;
                            (0..num_rows)
                                .into_iter()
                                .map(|row_idx| {
                                    view! {
                                        <tr>
                                            <th class="sticky left-0 cursor-pointer bg-white dark:bg-secondary-800">
                                                {core::utils::index_to_row(row_idx)}
                                            </th>
                                            {(0..num_cols)
                                                .into_iter()
                                                .map(|col_idx| {
                                                    let idx: core::data::CellIndex = (row_idx, col_idx).into();
                                                    match sheet.cells.read().get(&idx) {
                                                        Some(data) => {
                                                            EitherOf3::A(
                                                                view! { <CellValueData data=data.clone() idx=idx /> },
                                                            )
                                                        }
                                                        None => {
                                                            let domain = state::FormulaDomain::Cell {
                                                                workbook: workbook_id.clone(),
                                                                sheet: sheet_id.clone(),
                                                                cell: idx.clone(),
                                                            };
                                                            if let Some(formula) = formulas
                                                                .get_by_containing_domain(&domain)
                                                            {
                                                                EitherOf3::B(view! { <CellValueFormula formula idx=idx /> })
                                                            } else {
                                                                EitherOf3::C(view! { <CellEmpty idx /> })
                                                            }
                                                        }
                                                    }
                                                })
                                                .collect::<Vec<_>>()}
                                        </tr>
                                    }
                                })
                                .collect::<Vec<_>>()
                        }
                    }
                </tbody>
            </table>
        </div>
    }
}

const STATIC_CELL_DATA_CLASS: &'static str =
    "cursor-pointer hover:bg-secondary-50 dark:hover:bg-secondary-700";

#[component]
fn CellValueData(data: lib::data::Data, idx: core::data::CellIndex) -> impl IntoView {
    view! {
        <td class=STATIC_CELL_DATA_CLASS data-row=idx.row() ata-col=idx.col()>
            {calamine_data_to_string(&data)}
        </td>
    }
}

#[component]
fn CellValueFormula(formula: state::Formula, idx: core::data::CellIndex) -> impl IntoView {
    let state = expect_context::<state::State>();
    tracing::trace!("formula");
    let value = Signal::derive(move || {
        Result::<core::expr::Value, core::expr::Error>::Ok(core::expr::Value::String(
            "test".to_string(),
        ))
    });

    let is_err = move || value.read().is_err();

    let value_str = move || {
        value.with(|value| match value {
            Ok(value) => expr_value_to_string(value),
            Err(err) => expr_error_to_string(err),
        })
    };

    view! {
        <td
            class="cursor-pointer hover:bg-secondary-50 dark:hover:bg-secondary-700 border border-primary-600"
            class:bg-brand-red-500=is_err
            data-row=idx.row()
            data-col=idx.col()
        >
            {value_str}
        </td>
    }
}

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
