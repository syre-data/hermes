//! File explorer.
pub use active::ActiveFiles;
pub use nav::FileTree;

mod active {
    use crate::{LEVEL_PAD, LEVEL_PAD_UNIT, icon, state, state::FileResource, types};
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
                    <h2 class="font-bold uppercase">"Active files"</h2>
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
            let active = state.active_dataset.read_only();
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
            let active = state.active_dataset;
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
            let datasets = state.datasets;
            let selected = state.selected_files;
            let active = state.active_dataset;
            let id = file.id().clone();
                let fs_location = file.fs_location();
            move |e: ev::MouseEvent| {
                if e.button() != types::MouseButton::Primary {
                    return;
                }
                e.stop_propagation();

                if active.with_untracked(|active| {
                    active.as_ref().map(|active| *active == id).unwrap_or(false)
                }) {
                    // let idx = selected
                    //     .read_untracked()
                    //     .iter()
                    //     .position(|selected| *selected == id)
                    //     .expect("file is selected");
                    // let remaining_len = selected.read_untracked().len() - 1;
                    // if remaining_len == 0 {
                    //     active.write().take();
                    // } else if idx == remaining_len {
                    //     let next = selected
                    //         .read_untracked()
                    //         .get(remaining_len - 1)
                    //         .expect("file is last element")
                    //         .clone();
                    //     active.write().insert(next);
                    // } else {
                    //     let next = selected
                    //         .read_untracked()
                    //         .get(idx + 1)
                    //         .expect("file is not last element")
                    //         .clone();
                    //     active.write().insert(next);
                    // }
                    if let Some(next) = next_active_if_removed(active.read_only(), selected.read_only()) {
                        active.write().insert(next);
                    } else {
                        active.write().take();
                    }
                }

                 selected.update(|selected| {
                    selected.retain(|rid| *rid != id);
                });

                if matches!(fs_location, state::FsResourceLocation::FileSystem) {
                    datasets.update(|datasets| datasets.retain(|dataset| *dataset.file() != id));
                }
            }
        };

        let name_class = {
            let fs_location = file.fs_location();
            if matches!(fs_location, state::FsResourceLocation::App) {
            "grow text-primary-700 dark:text-primary-500"
        } else {
            "grow"
        }};

        view! {
            <div
                class="flex gap-2 items-end px cursor-pointer group/file text-nowrap"
                class=(["bg-secondary-50", "dark:bg-secondary-700"], is_active.clone())
                style:padding-left=format!("{LEVEL_PAD}{LEVEL_PAD_UNIT}")
                on:mousedown=activate
            >
                <div class=name_class>{name}</div>
                <small
                    class="truncate text-secondary-700 dark:text-secondary-200"
                    title=path.clone()
                >
                    {path.clone()}
                </small>
                <div>
                    <button class="hidden group-hover/file:block btn-cmd btn-secondary">
                        <Icon icon=icon::Close on:mousedown=remove />
                    </button>
                </div>
            </div>
        }
    }

    /// Get the `ResourceId` of the next active dataset if the current one is removed.
    /// If there is not a currently active dataset, returns `None`.
    pub fn next_active_if_removed( active: ReadSignal<state::ActiveDataset>, selected: ReadSignal<Vec<lib::ResourceId>>) -> Option<lib::ResourceId>{
        active.with_untracked(|active| {
        let id = active.as_ref()?;
        let idx = selected
                        .read_untracked()
                        .iter()
                        .position(|selected| selected == id)
                        .expect("file is selected");
                    let remaining_len = selected.read_untracked().len() - 1;
                    if remaining_len == 0 {
                        None
                    } else if idx == remaining_len {
                        let next = selected
                            .read_untracked()
                            .get(remaining_len - 1)
                            .expect("file is last element")
                            .clone();
                        Some(next)
                    } else {
                        let next = selected
                            .read_untracked()
                            .get(idx + 1)
                            .expect("file is not last element")
                            .clone();
                        Some(next)
                    }
                })         
    }
}

mod nav {
    use crate::{LEVEL_PAD, LEVEL_PAD_UNIT, icon, message, state, state::FileResource, types};
    use hermes_desktop_lib as lib;
    use leptos::{either::either, ev, html, prelude::*};
    use leptos_icons::Icon;
    use std::{io, path::PathBuf};

    const ROOT_PATH: &str = "/";

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

        let files = root.files.read_only();
        view! {
            <div class=root_class>
                <ProjectTitle class="font-bold pb" />
                <div>
                    <CreationSlot directory=state.directory_tree.root().clone() />
                    <div>
                        <For each=children key=|child| child.id().clone() let:child>
                            <DirectorySubtree directory=child level=1 />
                        </For>
                    </div>
                    <div>
                        <For each=files key=|file| file.id().clone() let:file>
                            <File file parent=root.clone() level=0 />
                        </For>
                    </div>
                </div>
            </div>
        }
    }

    #[component]
    fn ProjectTitle(#[prop(optional, into)] class: Option<String>) -> impl IntoView {
        let state = expect_context::<state::State>();

        let root_path = state.root_path().to_string_lossy().to_string();
        let root = state.directory_tree.root();
        let name = {
            let name = root.name.read_only();
            move || name.with(|name| name.to_string_lossy().to_string())
        };

        let root_class = if let Some(class) = class {
            format!("flex group {}", class)
        } else {
            "flex group".to_string()
        };

        view! {
            <div class=root_class>
                <div class="grow font-bold uppercase" title=root_path>
                    {name}
                </div>
                <div class="invisible group-hover:visible">
                    <commands::DirectoryCommands path=ROOT_PATH />
                </div>
            </div>
        }
    }

    #[component]
    fn CreationSlot(directory: state::Directory) -> impl IntoView {
        let state = expect_context::<state::State>();

        let path = state.directory_tree.get_directory_path(directory.id()).expect("directory should exist");
        let creation_slot = state.directory_tree.creation_slot.read_only();
        let directory = directory.clone();
        move || {
            let creation_slot = creation_slot.get();
            if matches!(
                creation_slot,
                state::DirectoryTreeCreationSlot::File { parent } if parent == path
            ) {
                Some(view! { <commands::NewFile parent=directory.clone() /> })
            } else {
                None
            }
        }
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
                    <CreationSlot directory=directory.clone() />
                    <div>
                        <For each=children key=|child| child.id().clone() let:child>
                            <DirectorySubtree directory=child.clone() level=level + 1 />
                        </For>
                    </div>
                    <div>
                        <For each=directory.files.read_only() key=|file| file.id().clone() let:file>
                            <File file parent=directory.clone() level />
                        </For>
                    </div>
                </div>
            </div>
        }
        .into_any()
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
        let state = expect_context::<state::State>();

        let name = {
            let name = directory.name.read_only();
            move || name.with(|name| name.to_string_lossy().to_string())
        };

        let path = state.directory_tree.get_directory_path(directory.id()).expect("directory to exist");

        view! {
            <div class="flex group">
                <div class="grow">{name}</div>
                <div class="invisible group-hover:visible">
                    <commands::DirectoryCommands path />
                </div>
            </div>
        }
    }

    #[component]
    fn File(file: state::File, parent: state::Directory, level: usize) -> impl IntoView {
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

        let file_content_view = either! {
            file.fs_location(),
            state::FsResourceLocation::FileSystem => view! { <FileContentForFileSystemFile file /> },
            state::FsResourceLocation::App =>view! { <FileContentForAppFile file parent=parent.clone()/> },
        };

        let inner = html::div()
            .style(("padding-left", format!("{LEVEL_PAD}{LEVEL_PAD_UNIT}")))
            .class(format!(
                "border-l border-l-transparent group-hover/level-{level}:border-secondary-100 \
                dark:group-hover/level-{level}:border-secondary-600 text-nowrap",
            ))
            .child(file_content_view);

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
    fn FileContentForAppFile(file: state::File, parent: state::Directory) -> impl IntoView {
        let state = expect_context::<state::State>();

        let load_dataset = {
            let file_id = file.id().clone();
            let selected = state.selected_files;
            let active = state.active_dataset;
            move |e: ev::MouseEvent| {
                if e.button() != types::MouseButton::Primary {
                    return;
                }

                if !selected.read_untracked().contains(&file_id) {
                    selected.write().push(file_id.clone());
                }
                if active
                    .read_untracked()
                    .as_ref()
                    .map(|active| *active != file_id)
                    .unwrap_or(true)
                {
                    active.write().insert(file_id.clone());
                }
            }
        };

        let remove = {
            let datasets = state.datasets;
            let selected = state.selected_files;
            let active = state.active_dataset;
            let formulas = state.formulas;
            let files = parent.files;
            let id = file.id().clone();
             move |e: ev::MouseEvent| {
                if e.button() != types::MouseButton::Primary {
                    return;
                }
                e.stop_propagation();

                if active.with_untracked(|active| {
                    active.as_ref().map(|active| *active == id).unwrap_or(false)
                }) {
                    if let Some(next) = super::active::next_active_if_removed(active.read_only(), selected.read_only()) {
                        active.write().insert(next);
                    } else {
                        active.write().take();
                    }
                }

                selected.update(|selected| {
                    selected.retain(|rid| *rid != id);
                });

                datasets.update(|datasets| datasets.retain(|dataset| *dataset.file() != id));
                files.update(|files| files.retain(|file| *file.id() != id));
                update_formulas_on_file_removal(formulas, &id);
            }
        };

        let name = {
            let name = file.name.read_only();
            move || name.with(|name| name.to_string_lossy().to_string())
        };

        view! {
            <div on:mousedown=load_dataset class="flex group">
                <div class="grow truncate text-primary-700 dark:text-primary-500">{name}</div>
                <div class="flex hidden group-hover:block">
                    <button type="button" class="block btn-cmd" on:mousedown=remove>
                        <Icon icon=icon::Remove />
                    </button>
                </div>
            </div>
        }
    }

    #[component]
    fn FileContentForFileSystemFile(file: state::File) -> impl IntoView {
        let state = expect_context::<state::State>();
        let load_dataset_action_abort_handle =
            expect_context::<state::LoadWorkbookActionAbortHandle>();

        let try_load_dataset = Action::new_local({
            let directory_tree = state.directory_tree.clone();
            let root_path = state.root_path().clone();
            let datasets = state.datasets;
            let selected = state.selected_files;
            let active = state.active_dataset;
            let messages = state.messages;
            let file_id = file.id().clone();
            move |_| {
                let directory_tree = directory_tree.clone();
                let root_path = root_path.clone();
                let file_id = file_id.clone();
                async move {
                    let path = directory_tree.get_file_path(&file_id).expect("file exists");
                    let path = root_path.join(path);
                    match load_dataset(path).await {
                        Ok(dataset) => {
                            datasets
                                .write()
                                .push(state::Dataset::new(file_id.clone(), dataset));

                            if !selected.read_untracked().contains(&file_id) {
                                selected.write().push(file_id.clone());
                            }
                            if active
                                .read_untracked()
                                .as_ref()
                                .map(|active| *active != file_id)
                                .unwrap_or(true)
                            {
                                active.write().insert(file_id.clone());
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
                                        hermes_desktop_lib::data::error::LoadCsv::DataTooLarge => {
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

        let dispatch_load_dataset = {
            let try_load_dataset_pending = try_load_dataset.pending();
            let mut dataset_abort_handle = load_dataset_action_abort_handle.clone();
            move |e: ev::MouseEvent| {
                if e.button() != types::MouseButton::Primary {
                    return;
                }
                if try_load_dataset_pending.get_untracked() {
                    return;
                }

                if let Some(other_pending) = dataset_abort_handle.take() {
                    other_pending.abort();
                }
                let abort_handle = try_load_dataset.dispatch(());
                dataset_abort_handle.insert(abort_handle);
            }
        };

        let abort_load_dataset = {
            let pending = try_load_dataset.pending();
            let mut abort_handle = load_dataset_action_abort_handle.clone();
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
            <div on:mousedown=dispatch_load_dataset class="flex">
                <div class="grow">{name}</div>
                {
                    let wb_load_pending = try_load_dataset.pending();
                    let abort_load_dataset = abort_load_dataset.clone();
                    move || {
                        wb_load_pending
                            .get()
                            .then_some(
                                view! {
                                    <div>
                                        <button
                                            on:mousedown=abort_load_dataset.clone()
                                            class="cursor-pointer"
                                        >
                                            <span class="block animate-spin">
                                                <Icon icon=icon::LoadingSpinner />
                                            </span>
                                        </button>
                                    </div>
                                },
                            )
                    }
                }
            </div>
        }
    }

    async fn load_dataset(path: PathBuf) -> Result<lib::data::Dataset, lib::data::error::Load> {
        #[derive(serde::Serialize)]
        struct Args {
            path: PathBuf,
        }

        tauri_sys::core::invoke_result("load_dataset", Args { path }).await
    }

    fn update_formulas_on_file_removal(formulas: state::Formulas, file: &lib::ResourceId) {
        formulas.update(|formulas| {
            formulas.retain(|formula|
                formula.domain.with_untracked(|domain| {
                    match domain {
                        state::FormulaDomain::CsvCell { dataset, cell : _} => {
                            dataset != file
                        },
                        state::FormulaDomain::WorkbookCell { dataset, sheet:_, cell:_ } => {
                            dataset != file
                        },
                    }
                })
            )
        });
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
            io::ErrorKind::Other => "Unknown.",
            err => {
                tracing::warn!(?err);
                "Unknown."
            }
        }
    }

    mod commands {
        use crate::{icon, state, types};
        use hermes_desktop_lib as lib;
        use leptos::{ev, html, prelude::*};
        use leptos_icons::Icon;
        use std::path::{Path, PathBuf};
        use wasm_bindgen::JsCast;

        enum DatasetFormat {
            Csv,
            Excel,
            OpenDoc,
        }

        impl DatasetFormat {
            pub fn from_str(input: impl AsRef<Path>) -> Option<Self> {
                let input = input.as_ref();
                let Some(ext) = input.extension() else {
                    return None;
                };

                let ext = ext.to_string_lossy();
                let ext = ext.to_ascii_lowercase();
                match ext.as_str() {
                    "csv" | "tsv" => Some(Self::Csv),
                    "xls" | "xlsx" => Some(Self::Excel),
                    "odt" => Some(Self::OpenDoc),
                    _ => None,
                }
            }
        }

        #[component]
        pub fn DirectoryCommands(path: impl Into<PathBuf>) -> impl IntoView {
            let state = expect_context::<state::State>();

            let add_file = {
                let path = path.into();
                let directory_tree = state.directory_tree.clone();
                move |e: ev::MouseEvent| {
                    if e.button() != types::MouseButton::Primary {
                        return;
                    }

                    if directory_tree.creation_slot.with_untracked(|creation_slot|  {                   
                        matches!(creation_slot, state::DirectoryTreeCreationSlot::File { parent } if *parent == path)}
                    ) {
                        return;
                    }

                    directory_tree
                        .creation_slot
                        .set(state::DirectoryTreeCreationSlot::File {
                            parent: path.clone(),
                        });
                }
            };

            view! {
                <div class="flex gap-2">
                    <button
                        on:mousedown=add_file
                        type="button"
                        title="Add a new file"
                        class="block btn-cmd cursor-pointer hover:bg-secondary-50 dark:hover:bg-secondary-700"
                    >
                        <Icon icon=icon::NewFile />
                    </button>
                </div>
            }
        }

        #[component]
        pub fn NewFile(parent: state::Directory) -> impl IntoView {
            const MAX_FILENAME_LENGTH: usize = 255;

            let state = expect_context::<state::State>();
            let owner = expect_context::<state::WorkspaceOwner>();

            let (filename, set_filename) = signal("".to_string());
            let (dirty, set_dirty) = signal(false);
            let (error, set_error) = signal(None);
            let input_node = NodeRef::<html::Input>::new();

            let _focus_cb = std::cell::OnceCell::new();
            Effect::new({
                move || {
                    if let Some(input) = input_node.get() {
                        let window = web_sys::window().unwrap();
                        let cb = wasm_bindgen::closure::Closure::<dyn Fn()>::new(move || {
                            let _ = input.focus();
                        });
                        let _ = window.set_timeout_with_callback(cb.as_ref().unchecked_ref());
                        _focus_cb.set(cb).unwrap();
                    }
                }
            });

            let oninput = move |_| {
                set_dirty(true);
            };

            let onblur = {
                let directory_tree = state.directory_tree.clone();
                let id = parent.id().clone();
                move |_| {
                    if filename.read_untracked().is_empty() {
                        let Some(path)  = directory_tree.get_directory_path(&id) else {
                        directory_tree.creation_slot.set(state::DirectoryTreeCreationSlot::None);
                          return;
                        };

                        if directory_tree.creation_slot.with_untracked(|slot|{
                            matches!(slot, state::DirectoryTreeCreationSlot::File { parent: path })
                        }
                        ) {
                        directory_tree.creation_slot.set(state::DirectoryTreeCreationSlot::None);
                        }
                    }
                } 
            };

            /// Validates a filename and sets the error message.
            let validate_filename = {
                let files = parent.files.read_only();
                move |filename: &String| {
                    set_error(None);
                    let dirty = dirty.get_untracked();

                    if filename.is_empty() || filename.len() > MAX_FILENAME_LENGTH {
                        if dirty {
                            set_error(Some(format!("must be between 1 and {MAX_FILENAME_LENGTH} characters")));
                        }
                        return false;
                    }

                    if files
                        .read_untracked()
                        .iter()
                        .any(|file| file.name.with_untracked(|name| **name == **filename))
                    {
                        if dirty {
                            set_error(Some("name already exists".to_string()));
                        }
                        return false;
                    }

                    if DatasetFormat::from_str(&filename).is_none() {
                        if dirty {
                            set_error(Some("invalid file extension".to_string()));
                        }
                        return false;
                    }

                    true
                }
            };

            let add_file = {
                let valid_filename = validate_filename.clone();
                let directory_tree = state.directory_tree.clone();
                let datasets = state.datasets;
                move |e: ev::SubmitEvent| {
                    e.prevent_default();
                    let filename = filename.get_untracked();
                    if !valid_filename(&filename) {
                        return;
                    }
                    
                    let dataset_format = DatasetFormat::from_str(&filename).expect("filename to be valid");
                    directory_tree
                        .creation_slot
                        .set(state::DirectoryTreeCreationSlot::None);
                    let file = owner.with(|| state::File::new_from_app(filename));
                    let file_id = file.id().clone();
                    parent.files.write().push(file);

                    let dataset = match dataset_format {
                        DatasetFormat::Csv => {
                            owner.with(||state::Csv::new(file_id.clone(), lib::data::Csv::new()).into())
                        }
                        DatasetFormat::Excel | DatasetFormat::OpenDoc => {
                            owner.with(||state::Workbook::new(file_id.clone(), lib::data::Workbook::new()).into())
                        }
                    };
                    state.datasets.write().push(dataset);

                    state.active_dataset.write().insert(file_id);
                }
            };

            view! {
                <form on:submit=add_file class="w-full">
                    <div class="px-2 py-1">
                        <input
                            node_ref=input_node
                            type="text"
                            bind:value=(filename, set_filename)
                            on:blur=onblur
                            on:input=oninput
                            class:ring-brand-red-600=move || {
                                filename.with(|filename| !validate_filename(filename))
                            }
                            class="w-full px-px ring invalid:ring-brand-red-600 focus-visible:invalid:ring-brand-red-600"
                            minlength="1"
                            maxlength=MAX_FILENAME_LENGTH
                        />
                        <small class="text-brand-red-600">{error}</small>
                    </div>
                </form>
            }
        }
    }
}
