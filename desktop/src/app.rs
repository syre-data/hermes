use crate::{component, dataset, explorer, formula, icon, message, state, types};
use hermes_core as core;
use hermes_desktop_lib as lib;
use leptos::{either::Either, ev, prelude::*, task};
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
    use futures::StreamExt;

    let state = state::State::new(root, graph);
    provide_context(state.clone());
    provide_context(state::LoadWorkbookActionAbortHandle::new());
    provide_context(state::WorkspaceOwner::with_current());
    provide_context(state::FormulaEditorVisibility::new());

    task::spawn_local_scoped_with_cancellation({
        let state = state.clone();
        async move {
            let mut app_events = tauri_sys::event::listen::<
                Vec<Result<lib::event::Event, lib::event::Error>>,
            >(lib::event::EVENT_TOPIC)
            .await
            .expect("could not create file system event listener");

            while let Some(event) = app_events.next().await {
                let events = event.payload;
                event::handle_events(events, &state);
            }
        }
    });

    view! {
        <div class="flex flex-col h-full">
            <div class="grow flex h-full">
                <div class="grow min-w-0 h-full">
                    <dataset::Workspace />
                </div>
                <component::ResizablePane>
                    <run::Run />
                    <formula::Workspace
                        {..}
                        class="border-l-secondary-50 dark:border-l-secondary-700 \
                        border-b-secondary-50 dark:border-b-secondary-700"
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
    use crate::{state, state::FileResource, types};
    use hermes_core as core;
    use hermes_desktop_lib as lib;
    use leptos::{ev, prelude::*};
    use std::{collections::HashMap, path::PathBuf};

    #[component]
    pub fn Run() -> impl IntoView {
        let state = expect_context::<state::State>();
        let formula_editor_vis = expect_context::<state::FormulaEditorVisibility>();

        let disabled = {
            let formulas = state.formulas.read_only();
            move || formulas.read().is_empty()
        };

        let run_workspace = Action::new_local({
            let state = state.clone();
            move |orders: &Vec<lib::formula::WorkspaceOrder>| {
                let orders = orders.clone();
                let state = state.clone();
                async move {
                    if let Err(errors) = run_workspace(&orders).await {
                        #[cfg(feature = "tracing")]
                        tracing::warn!(?errors);

                        let mut err_formulas = errors
                            .iter()
                            .flat_map(|err| err.formulas())
                            .collect::<Vec<_>>();
                        err_formulas.sort();
                        err_formulas.dedup();
                        state
                            .formulas
                            .write()
                            .retain(|formula| err_formulas.contains(&formula.id()));

                        if state.active_formula.with_untracked(|active| {
                            if let Some(active) = active.as_ref() {
                                !state
                                    .formulas
                                    .read()
                                    .iter()
                                    .any(|formula| formula.id() == active)
                            } else {
                                false
                            }
                        }) {
                            formula_editor_vis.set(false);
                            state.active_formula.set(None);
                        }
                    } else {
                        state.formulas.write().clear();
                        formula_editor_vis.set(false);
                        state.active_formula.set(None);
                    };
                }
            }
        });

        let dispatch_run_workspace = move |e: ev::MouseEvent| {
            if e.button() != types::MouseButton::Primary {
                return;
            }

            match formulas_to_workspace_orders(
                state.root_path().clone(),
                state.formulas,
                state.datasets,
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

    async fn run_workspace<'a>(
        orders: &'a Vec<lib::formula::WorkspaceOrder>,
    ) -> Result<(), Vec<lib::formula::error::WorkspaceOrder>> {
        #[derive(serde::Serialize)]
        struct Args<'a> {
            orders: &'a Vec<lib::formula::WorkspaceOrder>,
        }

        tauri_sys::core::invoke_result("run_workspace", Args { orders }).await
    }

    fn formulas_to_workspace_orders(
        root_path: PathBuf,
        formulas: state::Formulas,
        datasets: state::Datasets,
        directory_tree: state::DirectoryTree,
    ) -> Result<Vec<lib::formula::WorkspaceOrder>, Vec<error::InvalidCellValue>> {
        let (orders, errors) = sort_formulas_by_dataset(formulas.get_untracked())
            .into_iter()
            .map(|(ds_id, formulas)| {
                let dataset = datasets
                    .read_untracked()
                    .iter()
                    .find(|ds| *ds.id() == ds_id)
                    .expect("dataset should exist")
                    .clone();

                match dataset {
                    state::Dataset::Csv(csv) => {
                        let (formulas, errors) = formulas
                            .into_iter()
                            .map(|formula| dataset_csv_formula_to_workspace_update(formula, &csv))
                            .partition::<Vec<_>, _>(|res| res.is_ok());

                        if errors.is_empty() {
                            let formulas = formulas
                                .into_iter()
                                .map(|formula| formula.unwrap())
                                .collect::<Vec<_>>();

                            let path = directory_tree
                                .get_file_path(csv.file())
                                .expect("dataset file path should exist");
                            let path = root_path.join(path);

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

                    state::Dataset::Workbook(workbook) => {
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

    fn sort_formulas_by_dataset(
        formulas: Vec<state::Formula>,
    ) -> HashMap<lib::ResourceId, Vec<state::Formula>> {
        let mut wb_formulas = HashMap::new();
        for formula in formulas {
            let wb_id = formula.domain.with_untracked(|domain| match domain {
                state::FormulaDomain::CsvCell { dataset, cell } => dataset.clone(),
                state::FormulaDomain::WorkbookCell { dataset, .. } => dataset.clone(),
            });

            let entry = wb_formulas.entry(wb_id).or_insert(vec![]);
            entry.push(formula);
        }
        wb_formulas
    }

    fn dataset_csv_formula_to_workspace_update(
        formula: state::Formula,
        csv: &state::Csv,
    ) -> Result<lib::formula::UpdateCsv, error::InvalidCellValue> {
        formula.domain.with_untracked(|domain| match domain {
            state::FormulaDomain::CsvCell {
                dataset: ds_id,
                cell,
            } => {
                assert_eq!(ds_id, csv.id());
                let state::CellValue::Variable(value) = csv
                    .sheet()
                    .cells
                    .read_untracked()
                    .get(cell)
                    .expect("cell should exist")
                    .clone()
                else {
                    #[cfg(feature = "tracing")]
                    tracing::debug!(?cell);
                    panic!("invalid cell value type");
                };

                let Ok(value) = value.get_untracked().unwrap() else {
                    return Err(error::InvalidCellValue(cell.clone()));
                };

                Ok(lib::formula::UpdateCsv::new(
                    formula.id().clone(),
                    cell.row(),
                    cell.col(),
                    value,
                ))
            }

            state::FormulaDomain::WorkbookCell { .. } => unreachable!(),
        })
    }

    pub mod error {
        use hermes_core as core;

        #[derive(Debug)]
        pub struct InvalidCellValue(pub core::data::CellIndex);
    }
}

mod event {
    use crate::state;
    use hermes_desktop_lib as lib;
    use leptos::prelude::{ReadUntracked, Update, WithUntracked};

    #[cfg_attr(feature = "tracing", tracing::instrument(level = "debug", skip_all))]
    pub fn handle_events(
        events: Vec<Result<lib::event::Event, lib::event::Error>>,
        state: &state::State,
    ) {
        #[cfg(feature = "tracing")]
        tracing::trace!(?events);

        for event in events {
            match event {
                Ok(event) => process_event(event, &state),
                Err(err) => process_error(err, &state),
            }
        }
    }

    fn process_error(err: lib::event::Error, state: &state::State) {
        match err {
            hermes_desktop_lib::event::Error::LoadDataset { path, error } => todo!(),
        }
    }

    fn process_event(event: lib::event::Event, state: &state::State) {
        match event {
            lib::event::Event::File(_) => process_event_file(event, state),
            lib::event::Event::Folder(_) => todo!(),
            lib::event::Event::Any(_) => todo!(),
        }
    }

    fn process_event_file(event: lib::event::Event, state: &state::State) {
        let lib::event::Event::File(kind) = &event else {
            panic!("invalid event kind");
        };

        match kind {
            lib::event::File::Created(_) => todo!(),
            lib::event::File::Removed(_) => todo!(),
            lib::event::File::Renamed { .. } => todo!(),
            lib::event::File::Moved { .. } => todo!(),
            lib::event::File::Modified { .. } => process_event_file_modified(event, state),
        }
    }

    fn process_event_file_modified(event: lib::event::Event, state: &state::State) {
        use typed_path::Utf8TypedPath;

        let lib::event::Event::File(lib::event::File::Modified { path, data }) = event else {
            panic!("invalid event kind");
        };

        let file = {
            let path = path.to_string_lossy();
            let root_path = state.root_path().to_string_lossy();
            let path = Utf8TypedPath::derive(&path);
            let root_path = Utf8TypedPath::derive(&root_path);

            let Ok(rel_path) = path.strip_prefix(root_path) else {
                return;
            };
            let rel_path = rel_path.with_unix_encoding();
            let rel_path = std::path::PathBuf::from(rel_path.as_str());
            state.directory_tree.get_file_by_path(rel_path)
        };
        let Some(file) = file else {
            return;
        };

        state.datasets.update(|datasets| {
            let Some(dataset) = datasets
                .iter_mut()
                .find(|dataset| dataset.id() == file.id())
            else {
                return;
            };

            match (data, dataset) {
                (lib::data::Dataset::Csv(update), state::Dataset::Csv(dataset)) => {
                    dataset.set_data(update.clone());
                }
                (lib::data::Dataset::Workbook(new), state::Dataset::Workbook(old)) => todo!(),
                _ => todo!("dataset kind changed"),
            }
        });
    }
}
