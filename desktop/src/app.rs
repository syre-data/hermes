use crate::{component, explorer, formula, icon, message, state, types, workbook};
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
    let load_graph = LocalResource::new({
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
                                load_graph
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
                <div class="grow min-w-0">
                    <workbook::Workspace />
                </div>
                <component::ResizablePane>
                    <Run />
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

#[component]
fn Run() -> impl IntoView {
    let state = expect_context::<state::State>();
    let disabled = {
        let formulas = state.formulas.read_only();
        move || formulas.read().is_empty()
    };

    let run = Action::new_local({
        let formulas = state.formulas;
        move |_| async move {
            // TODO
        }
    });

    let dispatch_run = move |e: ev::MouseEvent| {
        if e.button() != types::MouseButton::Primary {
            return;
        }

        run.dispatch(());
    };

    view! {
        <div class="text-center">
            <button type="button" on:mousedown=dispatch_run disabled=disabled>
                "Run"
            </button>
        </div>
    }
}
