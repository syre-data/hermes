use crate::{component, explorer, formula, icon, message, state, types, workbook};
use hermes_core as core;
use hermes_desktop_lib as lib;
use leptos::{either::Either, ev, prelude::*};
use leptos_icons::Icon;
use leptos_meta::*;
use leptos_use::use_preferred_dark;
use serde::Serialize;
use std::path::PathBuf;

#[component]
pub fn App() -> impl IntoView {
    leptos_meta::provide_meta_context();
    let prefers_dark_mode = use_preferred_dark();
    let (root_path, set_root_path) = signal(None);

    let html_class = move || if prefers_dark_mode() { "dark" } else { "" };

    view! {
        <Title formatter=|text| text text="Hermes" />
        <Html attr:class=html_class />
        <Body attr:class="h-screen font-secondary overflow-hidden dark:bg-secondary-800 dark:text-white select-none" />

        <div class="h-full">
            {move || match root_path.get() {
                None => Either::Left(view! { <SelectRootPath set_root_path /> }),
                Some(root_path) => Either::Right(view! { <Workspace root=root_path /> }),
            }}
        </div>
    }
}

#[component]
fn SelectRootPath(set_root_path: WriteSignal<Option<PathBuf>>) -> impl IntoView {
    let select_folder_action = Action::new_local(move |_| async move {
        let path = tauri_sys::core::invoke::<Option<PathBuf>>("select_folder", ()).await;
        set_root_path(path);
    });

    let select_folder = move |e: ev::MouseEvent| {
        if e.button() != types::MouseButton::Primary {
            return;
        }

        select_folder_action.dispatch(());
    };

    view! {
        <main class="py-12">
            <h1 class="text-xl font-primary text-center">"Hermes"</h1>
            <div class="flex justify-center py-4">
                <button on:mousedown=select_folder class="btn btn-primary cursor-pointer">
                    "Open a folder"
                </button>
            </div>
        </main>
    }
}

#[component]
fn Workspace(root: PathBuf) -> impl IntoView {
    let load_directory_tree = LocalResource::new({
        let root = root.clone();
        move || load_directory(root.clone())
    });

    view! {
        <main class="h-full">
            <Suspense fallback=Loading>
                <ErrorBoundary fallback=|errors| {
                    view! { <LoadError errors /> }
                }>
                    {
                        let root = root.clone();
                        move || Suspend::new({
                            let root = root.clone();
                            async move {
                                load_directory_tree
                                    .await
                                    .map(|graph| {
                                        view! { <WorkspaceView root graph /> }
                                    })
                            }
                        })
                    }
                </ErrorBoundary>
            </Suspense>
        </main>
    }
}

#[component]
fn Loading() -> impl IntoView {
    view! { <div class="p-2 text-center">"Loading folder."</div> }
}

#[component]
fn LoadError(errors: ArcRwSignal<Errors>) -> impl IntoView {
    view! {
        <div class="p-2 text-center">
            <div>"The project could not be loaded."</div>
            <div class="text-sm">{format!("{errors:?}")}</div>
        </div>
    }
}

#[component]
fn WorkspaceView(root: PathBuf, graph: lib::fs::DirectoryTree) -> impl IntoView {
    let state = state::State::new(root, graph);
    provide_context(state.clone());
    provide_context(state::LoadWorkbookActionAbortHandle::new());
    provide_context(state::WorkspaceOwner::with_current());
    provide_context(state::FormulaEditorVisibility::new());

    view! {
        <div class="flex flex-col h-full">
            <div class="grow flex h-full">
                <div class="grow min-w-0 h-full">
                    <workbook::Workspace />
                </div>
                <component::ResizablePane>
                    <run::Run />
                    <formula::Workspace
                        {..}
                        class="border-l-secondary-50 dark:border-l-secondary-700 \
                        border-b-secondary-50 dark:border-b-secondary-700"
                    />
                    <explorer::OutputFiles
                        {..}
                        class="border-l-secondary-50 dark:border-l-secondary-700 \
                        border-b border-b-secondary-50 dark:border-b-secondary-700"
                    />
                    <explorer::ActiveFiles
                        {..}
                        class="border-l-secondary-50 dark:border-l-secondary-700 \
                        border-b border-b-secondary-50 dark:border-b-secondary-700"
                    />
                    <explorer::FileTree class="border-l-secondary-50 dark:border-l-secondary-700" />
                </component::ResizablePane>
            </div>

        </div>
        <div class="absolute top-0">
            <message::Messages />
        </div>
    }
}

async fn load_directory(
    root: PathBuf,
) -> Result<lib::fs::DirectoryTree, lib::fs::error::FromFileSystem> {
    #[derive(Serialize)]
    struct Args {
        root: PathBuf,
    }

    tauri_sys::core::invoke_result("load_directory", Args { root }).await
}

mod run {
    use crate::{state, types};
    use hermes_core as core;
    use hermes_desktop_lib as lib;
    use leptos::{ev, prelude::*};
    use std::collections::HashMap;

    #[component]
    pub fn Run() -> impl IntoView {
        let state = expect_context::<state::State>();
        let disabled = {
            let formulas = state.formulas.read_only();
            move || formulas.read().is_empty()
        };

        let run_workspace = Action::new_local({
            move |orders: &Vec<lib::formula::WorkspaceOrder>| {
                let orders = orders.clone();
                async move {
                    if let Err(err) = run_workspace(&orders).await {
                        tracing::warn!(?err);
                    } else {
                        tracing::info!("workspace run complete");
                    };
                }
            }
        });

        let dispatch_run_workspace = move |e: ev::MouseEvent| {
            if e.button() != types::MouseButton::Primary {
                return;
            }

            match formulas_to_workspace_orders(
                state.formulas,
                state.workbooks,
                state.directory_tree.clone(),
            ) {
                Ok(orders) => {
                    run_workspace.dispatch(orders);
                }
                Err(errors) => todo!(),
            }
        };

        view! {
            <div class="text-center">
                <button
                    type="button"
                    class="btn"
                    class:cursor-pointer=move || !disabled()
                    class:cursor-not-allowed=disabled
                    on:mousedown=dispatch_run_workspace
                    disabled=disabled
                >
                    "Run"
                </button>
            </div>
        }
    }

    /// # Returns
    /// If an error occurs, returns a `Vec<(<order index>, <error>)>`.
    async fn run_workspace<'a>(
        orders: &'a Vec<lib::formula::WorkspaceOrder>,
    ) -> Result<(), Vec<(usize, lib::formula::error::WorkspaceOrder)>> {
        #[derive(serde::Serialize)]
        struct Args<'a> {
            orders: &'a Vec<lib::formula::WorkspaceOrder>,
        }

        tauri_sys::core::invoke_result("run_workspace", Args { orders }).await
    }

    fn formulas_to_workspace_orders(
        formulas: state::Formulas,
        workbooks: state::Workbooks,
        directory_tree: state::DirectoryTree,
    ) -> Result<Vec<lib::formula::WorkspaceOrder>, Vec<error::InvalidCellValue>> {
        let (orders, errors) = sort_formulas_by_workbook(formulas.get_untracked())
            .into_iter()
            .map(|(wb_id, formulas)| {
                let workbook = workbooks
                    .read_untracked()
                    .iter()
                    .find(|wb| *wb.id() == wb_id)
                    .expect("workbook should exist")
                    .clone();

                match workbook.kind() {
                    lib::data::WorkbookKind::Csv => {
                        let (formulas, errors) = formulas
                            .into_iter()
                            .map(|formula| {
                                workbook_csv_formula_to_workspace_update(formula, &workbook)
                            })
                            .partition::<Vec<_>, _>(|res| res.is_ok());

                        if errors.is_empty() {
                            let formulas = formulas
                                .into_iter()
                                .map(|formula| formula.unwrap())
                                .collect::<Vec<_>>();

                            let path = directory_tree
                                .get_file_path(workbook.id())
                                .expect("workbook file path should exist");

                            Ok(lib::formula::WorkspaceOrder::Update(lib::formula::Update {
                                path,
                                updates: lib::formula::Updates::Csv(formulas),
                            }))
                        } else {
                            let errors = errors
                                .into_iter()
                                .map(|err| err.unwrap_err())
                                .collect::<Vec<_>>();

                            Err(errors)
                        }
                    }

                    lib::data::WorkbookKind::Workbook => {
                        todo!();
                    }
                }
            })
            .partition::<Vec<_>, _>(|res| res.is_ok());

        if errors.is_empty() {
            let updates = orders
                .into_iter()
                .map(|order| order.unwrap())
                .collect::<Vec<_>>();

            Ok(updates)
        } else {
            // TODO: Need to indicate the workbook each set of errors comes from.
            let errors = errors
                .into_iter()
                .flat_map(|err| err.unwrap_err())
                .collect::<Vec<_>>();
            Err(errors)
        }
    }

    fn sort_formulas_by_workbook(
        formulas: Vec<state::Formula>,
    ) -> HashMap<state::ResourceId, Vec<state::Formula>> {
        let mut wb_formulas = HashMap::new();
        for formula in formulas {
            let wb_id = formula.domain.with_untracked(|domain| match domain {
                state::FormulaDomain::Cell {
                    workbook,
                    sheet,
                    cell,
                } => workbook.clone(),
            });

            let entry = wb_formulas.entry(wb_id).or_insert(vec![]);
            entry.push(formula);
        }
        wb_formulas
    }

    fn workbook_csv_formula_to_workspace_update(
        formula: state::Formula,
        workbook: &state::Workbook,
    ) -> Result<lib::formula::UpdateCsv, error::InvalidCellValue> {
        formula.domain.with_untracked(|domain| match domain {
            state::FormulaDomain::Cell {
                workbook: wb_id,
                sheet,
                cell,
            } => {
                assert_eq!(wb_id, workbook.id());
                let state::CellValue::Variable(value) = workbook.sheets.read_untracked()[0]
                    .cells
                    .read_untracked()
                    .get(cell)
                    .expect("cell should exist")
                    .clone()
                else {
                    panic!("invalid cell value type");
                };

                let Ok(value) = value.get_untracked().unwrap() else {
                    return Err(error::InvalidCellValue(cell.clone()));
                };

                Ok(lib::formula::UpdateCsv {
                    row: cell.row(),
                    col: cell.col(),
                    value,
                })
            }
        })
    }

    pub mod error {
        use hermes_core as core;

        #[derive(Debug)]
        pub struct InvalidCellValue(pub core::data::CellIndex);
    }
}
