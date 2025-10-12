//! File explorer.
pub use active::ActiveFiles;
pub use nav::FileTree;

mod active {
    use crate::{LEVEL_PAD, LEVEL_PAD_UNIT, icon, state, types};
    use hermes_desktop_lib as lib;
    use leptos::{ev, prelude::*};
    use leptos_icons::Icon;

    #[component]
    pub fn ActiveFiles() -> impl IntoView {
        let state = expect_context::<state::State>();
        let directory_tree = state.directory_tree.clone();

        view! {
            <div>
                <div class="pb">
                    <h2 class="font-bold uppercase">"Selected files"</h2>
                </div>
                <div>
                    <For each=state.selected_files.read_only() key=|id| id.clone() let:id>
                        {
                            let file = directory_tree.get_file_by_id(&id).expect("file exists");
                            view! { <File file /> }
                        }
                    </For>
                </div>
            </div>
        }
    }

    #[component]
    fn File(file: state::File) -> impl IntoView {
        let state = expect_context::<state::State>();

        let name = {
            let name = file.name.read_only();
            move || name.with(|name| name.to_string_lossy().to_string())
        };

        let path = {
            state
                .directory_tree
                .get_file_path(file.id())
                .expect("file exists")
                .to_string_lossy()
                .to_string()
        };

        let is_active = {
            let active = state.active_workbook.read_only();
            let id = file.id().clone();
            move || {
                active
                    .read()
                    .as_ref()
                    .map(|active| *active == id)
                    .unwrap_or(false)
            }
        };

        let activate = {
            let id = file.id().clone();
            let active = state.active_workbook;
            move |e: ev::MouseEvent| {
                if e.button() != types::MouseButton::Primary {
                    return;
                }

                if !active
                    .read_untracked()
                    .as_ref()
                    .map(|active| *active == id)
                    .unwrap_or(false)
                {
                    let _ = active.write().insert(id.clone());
                }
            }
        };

        let remove = {
            let workbooks = state.workbooks;
            let selected = state.selected_files;
            let active = state.active_workbook;
            let id = file.id().clone();
            move |e: ev::MouseEvent| {
                if e.button() != types::MouseButton::Primary {
                    return;
                }
                e.stop_propagation();

                if active.with_untracked(|active| {
                    active.as_ref().map(|active| *active == id).unwrap_or(false)
                }) {
                    let idx = selected
                        .read_untracked()
                        .iter()
                        .position(|selected| *selected == id)
                        .expect("file is selected");
                    let remaining_len = selected.read_untracked().len() - 1;
                    if remaining_len == 0 {
                        active.write().take();
                    } else if idx == remaining_len {
                        let next = selected
                            .read_untracked()
                            .get(remaining_len - 1)
                            .expect("file is last element")
                            .clone();
                        active.write().insert(next);
                    } else {
                        let next = selected
                            .read_untracked()
                            .get(idx + 1)
                            .expect("file is not last element")
                            .clone();
                        active.write().insert(next);
                    }
                }

                selected.update(|selected| {
                    selected.retain(|rid| *rid != id);
                });
                workbooks.update(|workbooks| workbooks.retain(|workbook| *workbook.file() != id));
            }
        };

        view! {
            <div
                class="flex gap-2 items-end px cursor-pointer group/file text-nowrap"
                class=(["bg-secondary-50", "dark:bg-secondary-700"], is_active.clone())
                style:padding-left=format!("{LEVEL_PAD}{LEVEL_PAD_UNIT}")
                on:mousedown=activate
            >
                <div>{name}</div>
                <small
                    class="truncate text-secondary-700 dark:text-secondary-200"
                    title=path.clone()
                >
                    {path.clone()}
                </small>
                <button class="hidden group-hover/file:block btn-cmd btn-secondary">
                    <Icon icon=icon::Close on:mousedown=remove />
                </button>
            </div>
        }
    }
}

mod nav {
    use crate::{LEVEL_PAD, LEVEL_PAD_UNIT, icon, message, state, types};
    use hermes_desktop_lib as lib;
    use leptos::{ev, html, prelude::*};
    use leptos_icons::Icon;
    use std::{io, path::PathBuf};

    #[component]
    pub fn FileTree(#[prop(optional)] class: Option<&'static str>) -> impl IntoView {
        let state = expect_context::<state::State>();
        let root = state.directory_tree.root();
        let children = {
            let children = state.directory_tree.children(root.id().clone());
            move || children.with(|children| children.as_ref().expect("directory exists").clone())
        };
        let root_class = match class {
            Some(class) => format!("group/level-0 overflow-auto scrollbar-thin h-full {class}"),
            None => "group/level-0 overflow-auto scrollbar-thin h-full".to_string(),
        };

        view! {
            <div class=root_class>
                <ProjectRoot {..} class="font-bold pb" />
                <div>
                    <div>
                        <For each=children key=|child| child.id().clone() let:child>
                            <DirectorySubtree directory=child level=1 />
                        </For>
                    </div>
                    <div>
                        <For each=root.files.read_only() key=|file| file.id().clone() let:file>
                            <File file level=0 />
                        </For>
                    </div>
                </div>
            </div>
        }
    }

    #[component]
    fn ProjectRoot() -> impl IntoView {
        let state = expect_context::<state::State>();

        let root = state.directory_tree.root();
        let name = {
            let name = root.name.read_only();
            move || name.with(|name| name.to_string_lossy().to_string())
        };

        view! { <div class="font-bold uppercase">{name}</div> }
    }

    #[component]
    fn DirectorySubtree(directory: state::Directory, level: usize) -> impl IntoView {
        debug_assert!(level > 0);
        let state = expect_context::<state::State>();

        let children = {
            let children = state.directory_tree.children(directory.id().clone());
            move || children.with(|children| children.as_ref().expect("directory exists").clone())
        };

        view! {
            <div class=format!("group/level-{level}")>
                <Directory directory=directory.clone() level />
                <div>
                    <div>
                        <For each=children key=|child| child.id().clone() let:child>
                            <DirectorySubtree directory=child.clone() level=level + 1 />
                        </For>
                    </div>
                    <div>
                        <For each=directory.files.read_only() key=|file| file.id().clone() let:file>
                            <File file level />
                        </For>
                    </div>
                </div>
            </div>
        }
    }

    #[component]
    fn Directory(directory: state::Directory, level: usize) -> impl IntoView {
        debug_assert!(level > 0);

        let parent_level = level - 1;
        let ancestors = (0..parent_level)
            .map(|level| {
                html::div()
                    .style(("padding-left", format!("{LEVEL_PAD}{LEVEL_PAD_UNIT}")))
                    .class(format!(
                        "border-l border-l-transparent group-hover/level-{level}:border-secondary-100 \
                        dark:group-hover/level-{level}:border-secondary-600",
                    ))
            })
            .collect::<Vec<_>>();

        let inner = html::div()
            .style(("padding-left", format!("{LEVEL_PAD}{LEVEL_PAD_UNIT}")))
            .class(format!(
                "border-l border-l-transparent group-hover/level-{parent_level}:border-secondary-100 \
                dark:group-hover/level-{parent_level}:border-secondary-600 text-nowrap",
            ))
            .child(view! { <DirectoryContent directory /> });

        ancestors
            .into_iter()
            .rev()
            .fold(inner, |child, parent| parent.child(child))
            .class(
                "border-l border-l-transparent group-hover/level-0:border-secondary-100 \
                dark:group-hover/level-0:border-secondary-600 \
                hover:bg-secondary-50 dark:hover:bg-secondary-700 cursor-pointer",
            )
    }

    #[component]
    fn DirectoryContent(directory: state::Directory) -> impl IntoView {
        let name = {
            let name = directory.name.read_only();
            move || name.with(|name| name.to_string_lossy().to_string())
        };

        view! { {name} }
    }

    #[component]
    fn File(file: state::File, level: usize) -> impl IntoView {
        let state = expect_context::<state::State>();

        let is_selected = {
            let selected = state.selected_files.read_only();
            let id = file.id().clone();
            move || selected.read().contains(&id)
        };

        let ancestors = (0..level)
            .map(|level| {
                html::div()
                    .style(("padding-left", format!("{LEVEL_PAD}{LEVEL_PAD_UNIT}")))
                    .class(format!(
                        "border-l border-l-transparent group-hover/level-{level}:border-secondary-100 \
                        dark:group-hover/level-{level}:border-secondary-600",
                    ))
                    })
            .collect::<Vec<_>>();

        let inner = html::div()
            .style(("padding-left", format!("{LEVEL_PAD}{LEVEL_PAD_UNIT}")))
            .class(format!(
                "border-l border-l-transparent group-hover/level-{level}:border-secondary-100 \
                dark:group-hover/level-{level}:border-secondary-600 text-nowrap",
            ))
            .child(view! { <FileContent file /> });

        ancestors
            .into_iter()
            .rev()
            .fold(inner, |child, parent| parent.child(child))
            .class(
                "border-l border-l-transparent group-hover/level-0:border-secondary-100 \
                dark:group-hover/level-0:border-secondary-600 \
                hover:bg-secondary-50 dark:hover:bg-secondary-700 cursor-pointer",
            )
            .class(("bg-secondary-50", is_selected.clone()))
            .class(("dark:bg-secondary-700", is_selected.clone()))
    }

    #[component]
    fn FileContent(file: state::File) -> impl IntoView {
        let state = expect_context::<state::State>();
        let load_wb_action_abort_handle = expect_context::<state::LoadWorkbookActionAbortHandle>();

        let try_load_workbook = Action::new_local({
            let directory_tree = state.directory_tree.clone();
            let root_path = state.root_path().clone();
            let workbooks = state.workbooks;
            let selected = state.selected_files;
            let active = state.active_workbook;
            let messages = state.messages;
            let id = file.id().clone();
            move |_| {
                let directory_tree = directory_tree.clone();
                let root_path = root_path.clone();
                let id = id.clone();
                async move {
                    let path = directory_tree.get_file_path(&id).expect("file exists");
                    let path = root_path.join(path);
                    match load_workbook(path).await {
                        Ok(workbook) => {
                            workbooks
                                .write()
                                .push(state::Workbook::new(id.clone(), workbook));

                            if !selected.read_untracked().contains(&id) {
                                selected.write().push(id.clone());
                            }
                            if active
                                .read_untracked()
                                .as_ref()
                                .map(|active| *active != id)
                                .unwrap_or(true)
                            {
                                active.write().insert(id.clone());
                            }
                        }
                        Err(err) => {
                            messages.update(|messages| {
                                let body = match err {
                                    hermes_desktop_lib::data::error::Load::InvalidFileType => {
                                        "Invalid file type"
                                    }
                                    hermes_desktop_lib::data::error::Load::Csv(err) => match err {
                                        hermes_desktop_lib::data::error::LoadCsv::Io(err) => {
                                            io_error_message(err)
                                        }
                                        hermes_desktop_lib::data::error::LoadCsv::TooLarge => {
                                            "File too large."
                                        }
                                    },
                                    hermes_desktop_lib::data::error::Load::Excel(err) => {
                                        match err {
                                            hermes_desktop_lib::data::error::LoadExcel::Io(err) => {
                                                io_error_message(err)
                                            }
                                        }
                                    }
                                };
                                let msg =
                                    message::Message::error_with_body("Could not load file.", body);
                                messages.push(msg);
                            });
                        }
                    }
                }
            }
        });

        let dispatch_load_workbook = {
            let try_load_wb_pending = try_load_workbook.pending();
            let mut wb_abort_handle = load_wb_action_abort_handle.clone();
            move |e: ev::MouseEvent| {
                if e.button() != types::MouseButton::Primary {
                    return;
                }
                if try_load_wb_pending.get_untracked() {
                    return;
                }

                if let Some(other_pending) = wb_abort_handle.take() {
                    other_pending.abort();
                }
                let abort_handle = try_load_workbook.dispatch(());
                wb_abort_handle.insert(abort_handle);
            }
        };

        let abort_load_workbook = {
            let pending = try_load_workbook.pending();
            let mut abort_handle = load_wb_action_abort_handle.clone();
            move |e: ev::MouseEvent| {
                if e.button() != types::MouseButton::Primary {
                    return;
                }
                if !pending.get_untracked() {
                    return;
                }
                if let Some(abort_handle) = abort_handle.take() {
                    abort_handle.abort();
                }
            }
        };

        let name = {
            let name = file.name.read_only();
            move || name.with(|name| name.to_string_lossy().to_string())
        };

        view! {
            <div on:mousedown=dispatch_load_workbook class="flex">
                <div class="grow">{name}</div>
                {
                    let wb_load_pending = try_load_workbook.pending();
                    let abort_load_workbook = abort_load_workbook.clone();
                    move || {
                        wb_load_pending
                            .get()
                            .then_some(
                                view! {
                                    <div>
                                        <button
                                            on:mousedown=abort_load_workbook.clone()
                                            class="cursor-pointer"
                                        >
                                            <Icon icon=icon::LoadingSpinner />
                                        </button>
                                    </div>
                                },
                            )
                    }
                }
            </div>
        }
    }

    async fn load_workbook(path: PathBuf) -> Result<lib::data::Workbook, lib::data::error::Load> {
        #[derive(serde::Serialize)]
        struct Args {
            path: PathBuf,
        }

        tauri_sys::core::invoke_result("load_workbook", Args { path }).await
    }

    fn io_error_message(err: io::ErrorKind) -> &'static str {
        match err {
            io::ErrorKind::NotFound => "File not found.",
            io::ErrorKind::PermissionDenied => "Permission denied.",
            io::ErrorKind::AlreadyExists => "File already exists.",
            io::ErrorKind::NotADirectory => "Not a directory.",
            io::ErrorKind::IsADirectory => "Is a directory.",
            io::ErrorKind::DirectoryNotEmpty => "Directory is not empty.",
            io::ErrorKind::FileTooLarge => "File is too large.",
            io::ErrorKind::InvalidFilename => "Invalid file name.",
            io::ErrorKind::UnexpectedEof => "Unexpected end of file.",
            io::ErrorKind::ConnectionRefused => todo!(),
            io::ErrorKind::ConnectionReset => todo!(),
            io::ErrorKind::HostUnreachable => todo!(),
            io::ErrorKind::NetworkUnreachable => todo!(),
            io::ErrorKind::ConnectionAborted => todo!(),
            io::ErrorKind::NotConnected => todo!(),
            io::ErrorKind::AddrInUse => todo!(),
            io::ErrorKind::AddrNotAvailable => todo!(),
            io::ErrorKind::NetworkDown => todo!(),
            io::ErrorKind::BrokenPipe => todo!(),
            io::ErrorKind::WouldBlock => todo!(),
            io::ErrorKind::ReadOnlyFilesystem => todo!(),
            io::ErrorKind::StaleNetworkFileHandle => todo!(),
            io::ErrorKind::InvalidInput => todo!(),
            io::ErrorKind::InvalidData => todo!(),
            io::ErrorKind::TimedOut => todo!(),
            io::ErrorKind::WriteZero => todo!(),
            io::ErrorKind::StorageFull => todo!(),
            io::ErrorKind::NotSeekable => todo!(),
            io::ErrorKind::QuotaExceeded => todo!(),
            io::ErrorKind::ResourceBusy => todo!(),
            io::ErrorKind::ExecutableFileBusy => todo!(),
            io::ErrorKind::Deadlock => todo!(),
            io::ErrorKind::CrossesDevices => todo!(),
            io::ErrorKind::TooManyLinks => todo!(),
            io::ErrorKind::ArgumentListTooLong => todo!(),
            io::ErrorKind::Interrupted => todo!(),
            io::ErrorKind::Unsupported => todo!(),
            io::ErrorKind::OutOfMemory => todo!(),
            io::ErrorKind::Other | _ => "Unknown.",
        }
    }
}
