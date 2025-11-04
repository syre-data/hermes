use crate::{dataset, icon, state, types};
use hermes_core as core;
use hermes_desktop_lib as lib;
use leptos::{either::Either, ev, html, prelude::*};
use leptos_icons::Icon;

#[component]
pub fn Workspace() -> impl IntoView {
    let state = expect_context::<state::State>();

    view! {
        <div>
            <div class="pb">
                <h2 class="font-bold uppercase">"Formulas"</h2>
            </div>
            <div>
                <For each=state.formulas.read_only() key=|formula| formula.id().clone() let:formula>
                    <Formula formula />
                </For>
            </div>
        </div>
    }
}

#[component]
fn Formula(formula: state::Formula) -> impl IntoView {
    let state = expect_context::<state::State>();
    let editor_vis = expect_context::<state::FormulaEditorVisibility>();

    let domain = formula.domain.read_only();
    let title = {
        let datasets = state.datasets;
        move || {
            domain.with(|domain| match domain {
                state::FormulaDomain::CsvCell { dataset, cell } => cell.to_string(),

                state::FormulaDomain::WorkbookCell {
                    dataset,
                    sheet,
                    cell,
                } => {
                    let dataset = datasets
                        .read_untracked()
                        .iter()
                        .find(|ds| ds.id() == dataset)
                        .expect("dataset to exist")
                        .clone();

                    match dataset {
                        state::Dataset::Csv(_) => unreachable!(),
                        state::Dataset::Workbook(workbook) => {
                            let sheet_name = workbook
                                .sheets
                                .read_untracked()
                                .iter()
                                .find_map(|wb_sheet| {
                                    (wb_sheet.id() == sheet).then_some(wb_sheet.name.read_only())
                                })
                                .expect("sheet to exist");

                            format!("{}!{cell}", sheet_name.get())
                        }
                    }
                }
            })
        }
    };

    let path = {
        let directory_tree = state.directory_tree;
        move || {
            domain.with(|domain| match domain {
                state::FormulaDomain::CsvCell { dataset, .. } => directory_tree
                    .get_file_path(dataset)
                    .expect("file to exist")
                    .to_string_lossy()
                    .to_string(),
                state::FormulaDomain::WorkbookCell { dataset, .. } => directory_tree
                    .get_file_path(dataset)
                    .expect("file to exist")
                    .to_string_lossy()
                    .to_string(),
            })
        }
    };

    let is_active = {
        let active_formula = state.active_formula;
        let id = formula.id().clone();
        move || {
            active_formula
                .with(|active| active.as_ref().map(|active| *active == id).unwrap_or(false))
        }
    };

    let set_as_active = {
        let active_formula = state.active_formula;
        let id = formula.id().clone();
        move |e: ev::MouseEvent| {
            if e.button() != types::MouseButton::Primary {
                return;
            }

            if active_formula.with_untracked(|active| {
                active.as_ref().map(|active| *active != id).unwrap_or(false)
            }) {
                editor_vis.set(true);
                let _ = active_formula.write().insert(id.clone());
            }
        }
    };

    let remove = {
        let formulas = state.formulas;
        let active_formula = state.active_formula;
        let id = formula.id().clone();
        move |e: ev::MouseEvent| {
            if e.button() != types::MouseButton::Primary {
                return;
            }
            e.stop_propagation();

            if active_formula.with_untracked(|active| {
                active.as_ref().map(|active| *active == id).unwrap_or(false)
            }) {
                editor_vis.set(false);
                active_formula.set(None);
            }
            formulas.update(|formulas| formulas.retain(|formula| *formula.id() != id));
        }
    };

    view! {
        <div
            class="flex items-end group/formula cursor-pointer hover:bg-secondary-50 dark:hover:bg-secondary-700"
            class=(["bg-secondary-50", "dark:bg-secondary-700"], is_active.clone())
            on:mousedown=set_as_active
        >
            <div class="grow font-bold">{title}</div>
            <small class="truncate text-secondary-700 dark:text-secondary-200" title=path.clone()>
                {path.clone()}
            </small>
            <div class="hidden group-hover/formula:block">
                <button type="button" class="btn-cmd btn-secondary" on:mousedown=remove>
                    <Icon icon=icon::Remove />
                </button>
            </div>
        </div>
    }
}

/// Group consecutive indices into groups returning a list of `(start, end)` indexes for each consecutive group.
///
/// # Examples
/// ```rust
/// let idx = vec![0, 2, 3, 5, 6, 7, 8, 9];
/// let grouped = collapse_indices(&idx);
/// assert_eq!(grouped, vec![(0, 0) (2, 3), (5, 9)]);
/// ```
fn group_indices(
    idx: &Vec<core::data::IndexType>,
) -> Vec<(core::data::IndexType, core::data::IndexType)> {
    if idx.is_empty() {
        return vec![];
    }

    let mut idx = idx.clone();
    idx.sort();
    idx.dedup();
    let mut groups = vec![];
    let mut start = idx[0];
    let mut prev = idx[0];
    for idx in idx.into_iter().skip(1) {
        if idx > prev + 1 {
            groups.push((start, prev));
            start = idx;
            prev = idx;
        } else {
            prev = idx;
        }
    }
    groups.push((start, prev));

    groups
}

#[component]
pub fn Editor() -> impl IntoView {
    let state = expect_context::<state::State>();
    let active_formula = state.active_formula.read_only();
    let formulas = state.formulas;

    move || {
        if let Some(formula) = active_formula.read().as_ref() {
            let formula = formulas.get(formula).expect("formula to exist");
            Either::Left(view! { <EditorEnabled formula /> })
        } else {
            Either::Right(view! { <EditorDisabled /> })
        }
    }
}

#[component]
fn EditorDisabled() -> impl IntoView {
    view! { <div>"Select a formula to edit."</div> }
}

#[component]
fn EditorEnabled(formula: state::Formula) -> impl IntoView {
    let state = expect_context::<state::State>();
    let workspace_owner = expect_context::<state::WorkspaceOwner>();
    let editor_vis = expect_context::<state::FormulaEditorVisibility>();
    let input_node = NodeRef::<html::Input>::new();

    Effect::new(move || {
        let Some(input) = input_node.get() else {
            return;
        };
        if editor_vis.get() {
            if let Err(err) = input.focus() {
                tracing::warn!(?err);
            };
        }
    });

    let (input, set_input) = signal(formula.value.get_untracked());
    let (error, set_error) = signal::<Option<&'static str>>(None);

    let save_formula = {
        let datasets = state.datasets;
        let formulas = state.formulas;
        let active_formula = state.active_formula;
        let formula = formula.clone();
        move || {
            input.with_untracked(|input| {
                let input = input.trim();
                if input.is_empty() {
                    formulas.update(|formulas| {
                        formulas.retain(|f| f.id() != formula.id());
                    });
                    active_formula.set(None);
                    editor_vis.set(false);
                } else {
                    match core::expr::parse(input) {
                        Ok(_expr) => {
                            set_error(None);
                            formula.value.set(input.to_string());
                            sync_formula(&formula, &datasets, &workspace_owner);
                        }
                        Err(err) => {
                            let msg = match err {
                                core::expr::Error::Tokenize(_kind) => "syntax error",
                                core::expr::Error::Parse(_kind) => "parse error",
                                _ => unreachable!("invalid error kind"),
                            };
                            set_error(Some(msg));
                        }
                    }
                }
            })
        }
    };

    let save_formula_trigger = {
        let save_formula = save_formula.clone();
        move |e: ev::SubmitEvent| {
            e.prevent_default();
            save_formula()
        }
    };

    let title = {
        let domain = formula.domain.read_only();
        let directory_tree = state.directory_tree.clone();
        let datasets = state.datasets;
        move || match domain.get() {
            state::FormulaDomain::CsvCell { dataset, cell } => cell.to_string(),

            state::FormulaDomain::WorkbookCell {
                dataset,
                sheet,
                cell,
            } => {
                let file = directory_tree
                    .get_file_by_id(&dataset)
                    .expect("file to exist");
                let dataset = datasets
                    .read()
                    .iter()
                    .find(|ds| *ds.id() == dataset)
                    .expect("dataset to exist")
                    .clone();
                match dataset {
                    state::Dataset::Csv(_) => unreachable!(),
                    state::Dataset::Workbook(workbook) => {
                        let sheet_name = workbook
                            .sheets
                            .read()
                            .iter()
                            .find_map(|s| (*s.id() == sheet).then_some(s.name.read_only()))
                            .expect("sheet to exist");
                        format!("{}!{cell}", sheet_name.get())
                    }
                }
            }
        }
    };

    view! {
        <div class="flex">
            <div>{title}</div>
            <form class="grow" on:submit=save_formula_trigger>
                <div>
                    <label
                        class="flex border border-transparent"
                        class:border-color-brand-red-600=move || error.read().is_some()
                    >
                        <Icon icon=icon::Equal />
                        <input
                            node_ref=input_node
                            name="formula"
                            type="text"
                            class="grow input-compact"
                            bind:value=(input, set_input)
                        />
                    </label>
                    <div>
                        <small class="color-brand-red-600">{error}</small>
                    </div>
                </div>
            </form>
        </div>
    }
}

/// Update workbook data for formula.
/// Creates a new cell if needed.
fn sync_formula(
    formula: &state::Formula,
    datasets: &state::Datasets,
    owner: &state::WorkspaceOwner,
) {
    formula.domain.with_untracked(|domain| match domain {
        state::FormulaDomain::CsvCell { dataset, cell } => datasets.with_untracked(|datasets| {
            let dataset = datasets
                .iter()
                .find(|ds| ds.id() == dataset)
                .expect("dataset should exist");

            let (cells, origin) = match dataset {
                state::Dataset::Csv(csv) => (
                    csv.sheet().cells,
                    core::data::CellPath {
                        sheet: 0,
                        row: cell.row(),
                        col: cell.col(),
                    },
                ),
                state::Dataset::Workbook(workbook) => unreachable!(),
            };

            let value = core::expr::eval(formula.value.get_untracked(), dataset, &origin);
            if cells.with_untracked(|cells| cells.contains_key(cell)) {
                cells.with_untracked(|cells| {
                    let state::CellValue::Variable(cell) = cells.get(cell).expect("cell to exist")
                    else {
                        panic!("expected a variable cell");
                    };
                    cell.set(state::VariableCellValue::Formula(
                        value.map(|value| value.into()),
                    ));
                });
            } else {
                cells.update(|cells| {
                    let state::CellValue::Variable(cell) =
                        cells
                            .entry(cell.clone())
                            .or_insert(state::CellValue::Variable(
                                owner.with(|| RwSignal::new(state::VariableCellValue::Empty)),
                            ))
                    else {
                        panic!("expected a formula cell");
                    };
                    cell.set(state::VariableCellValue::Formula(
                        value.map(|value| value.into()),
                    ));
                });
            }
        }),

        state::FormulaDomain::WorkbookCell {
            dataset,
            sheet,
            cell,
        } => datasets.with_untracked(|datasets| {
            let dataset = datasets
                .iter()
                .find(|ds| ds.id() == dataset)
                .expect("dataset should exist");

            let (cells, origin) = match dataset {
                state::Dataset::Csv(csv) => unreachable!(),
                state::Dataset::Workbook(workbook) => {
                    let (sheet_idx, cells) = workbook
                        .sheets
                        .read_untracked()
                        .iter()
                        .enumerate()
                        .find_map(|(idx, s)| (s.id() == sheet).then_some((idx, s.cells)))
                        .expect("sheet should exist");

                    (
                        cells,
                        core::data::CellPath {
                            sheet: sheet_idx as core::data::IndexType,
                            row: cell.row(),
                            col: cell.col(),
                        },
                    )
                }
            };

            let value = core::expr::eval(formula.value.get_untracked(), dataset, &origin);
            if cells.with_untracked(|cells| cells.contains_key(cell)) {
                cells.with_untracked(|cells| {
                    let state::CellValue::Variable(cell) = cells.get(cell).expect("cell to exist")
                    else {
                        panic!("expected a variable cell");
                    };
                    cell.set(state::VariableCellValue::Formula(
                        value.map(|value| value.into()),
                    ));
                });
            } else {
                cells.update(|cells| {
                    let state::CellValue::Variable(cell) =
                        cells
                            .entry(cell.clone())
                            .or_insert(state::CellValue::Variable(
                                owner.with(|| RwSignal::new(state::VariableCellValue::Empty)),
                            ))
                    else {
                        panic!("expected a formula cell");
                    };
                    cell.set(state::VariableCellValue::Formula(
                        value.map(|value| value.into()),
                    ));
                });
            }
        }),
    })
}
