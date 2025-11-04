use hermes_fs_daemon as fs_daemon;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::select_folder,
            commands::load_directory,
            commands::load_dataset,
            commands::run_workspace,
        ])
        .setup(setup)
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg_attr(feature = "tracing", tracing::instrument(skip(app)))]
fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let fs_daemon = fs_daemon::server::Builder::new();
    std::thread::Builder::new()
        .name("hermes desktop fs daemon".to_string())
        .spawn(move || fs_daemon.run())
        .unwrap();
}

mod commands {
    use hermes_core as core;
    use hermes_desktop_lib as lib;
    use std::path::PathBuf;
    use tauri_plugin_dialog::{DialogExt, FilePath};

    #[tauri::command]
    pub async fn select_folder(app: tauri::AppHandle) -> Option<PathBuf> {
        app.dialog()
            .file()
            .set_title("Choose a folder")
            .blocking_pick_folder()
            .map(|path| {
                let FilePath::Path(path) = path else {
                    panic!("invalid path kind");
                };
                path
            })
    }

    #[tauri::command]
    pub fn load_directory(
        root: PathBuf,
    ) -> Result<lib::fs::DirectoryTree, lib::fs::error::FromFileSystem> {
        lib::fs::DirectoryTree::from_file_system(root)
    }

    #[tauri::command]
    pub fn load_dataset(path: PathBuf) -> Result<lib::data::Dataset, lib::data::error::Load> {
        use lib::data::Dataset;

        let file_kind = if let Some(ext) = path.extension().map(|ext| ext.to_str()).flatten() {
            match ext {
                "csv" | "tsv" => FileKind::Csv,
                "xlsx" | "xls" => FileKind::Excel,
                _ => FileKind::Unknown,
            }
        } else {
            FileKind::Unknown
        };

        match file_kind {
            FileKind::Csv => lib::data::Csv::load_from_path(&path)
                .map(|csv| csv.into())
                .map_err(|err| err.into()),
            FileKind::Excel => lib::data::Workbook::load_from_path(&path)
                .map(|workbook| workbook.into())
                .map_err(|err| err.into()),
            FileKind::Unknown => match lib::data::Csv::load_from_path(&path) {
                Ok(csv) => Ok(csv.into()),
                Err(csv_err) => match csv_err {
                    lib::data::error::LoadCsv::Io(_) => Err(csv_err.into()),
                    _ => match lib::data::Workbook::load_from_path(&path) {
                        Ok(workbook) => Ok(workbook.into()),
                        Err(_) => Err(lib::data::error::Load::InvalidFileType),
                    },
                },
            },
        }
    }

    #[derive(Debug)]
    enum FileKind {
        Csv,
        Excel,
        Unknown,
    }

    /// Run workspace orders.
    ///
    /// # Returns
    /// If errors occur, returns a `Vec<(<order index>, <error>)>`.
    #[tauri::command]
    pub async fn run_workspace(
        orders: Vec<lib::formula::WorkspaceOrder>,
    ) -> Result<(), Vec<(usize, lib::formula::error::WorkspaceOrder)>> {
        let mut tasks = tokio::task::JoinSet::new();
        let mut task_handles = Vec::with_capacity(orders.len());
        for order in orders {
            let handle = tasks.spawn(run_workspace_order(order));
            task_handles.push(handle);
        }

        let mut errors = Vec::new();
        while let Some(result) = tasks.join_next_with_id().await {
            match result {
                Ok((id, result)) => {
                    if let Err(err) = result {
                        let idx = task_handles
                            .iter()
                            .position(|handle| handle.id() == id)
                            .expect("task handle should exist");

                        errors.push((idx, err))
                    }
                }

                Err(err) => {
                    let idx = task_handles
                        .iter()
                        .position(|handle| handle.id() == err.id())
                        .expect("task handle should exist");

                    errors.push((idx, lib::formula::error::WorkspaceOrder::TaskNotCompleted));
                }
            }
        }

        if errors.is_empty() {
            return Ok(());
        } else {
            return Err(errors);
        }
    }

    async fn run_workspace_order(
        order: lib::formula::WorkspaceOrder,
    ) -> Result<(), lib::formula::error::WorkspaceOrder> {
        match order {
            lib::formula::WorkspaceOrder::Create => todo!(),
            lib::formula::WorkspaceOrder::Update(update) => {
                run_workspace_order_update(update).await
            }
        }
    }

    async fn run_workspace_order_update(
        update: lib::formula::Update,
    ) -> Result<(), lib::formula::error::WorkspaceOrder> {
        let lib::formula::Update { path, updates } = update;
        match updates {
            lib::formula::Updates::Csv(updates) => {
                run_workspace_order_update_csv(path, updates).await
            }
            lib::formula::Updates::Workbook(updates) => {
                run_workspace_order_update_workbook(path, updates).await
            }
        }
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(level = "trace"))]
    async fn run_workspace_order_update_csv(
        path: PathBuf,
        updates: Vec<lib::formula::UpdateCsv>,
    ) -> Result<(), lib::formula::error::WorkspaceOrder> {
        #[cfg(feature = "tracing")]
        tracing::trace!("processing orders");

        let file = tokio::fs::File::open(&path)
            .await
            .map_err(|err| lib::formula::error::WorkspaceOrder::OpenFile(err.kind()))?
            .into_std()
            .await;
        let rdr = csv::Reader::from_reader(file);
        let mut csv = lib::data::Csv::from_csv_reader(rdr)?;
        for update in updates {
            let idx = core::data::CellIndex::new(update.row, update.col);
            csv.sheet
                .insert(idx, update.value)
                .expect("cell should be empty");
        }

        csv.save(&path)?;
        Ok(())
    }

    async fn run_workspace_order_update_workbook(
        path: PathBuf,
        updates: Vec<lib::formula::UpdateWorkbook>,
    ) -> Result<(), lib::formula::error::WorkspaceOrder> {
        todo!();
    }
}
