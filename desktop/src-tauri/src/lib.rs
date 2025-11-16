use hermes_fs_daemon as fs_daemon;
use tauri::Manager;

mod fs_event;

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

/// Runs setup tasks:
/// 1. Launches `fs_daemon`.
/// 2. Registers event listeners.
#[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let (command_tx, command_rx) = fs_daemon::server::command_channel();
    let (event_tx, event_rx) = fs_daemon::server::event_channel();
    let mut daemon = fs_daemon::server::Daemon::new(event_tx, command_rx);

    let daemon_handle = std::thread::Builder::new()
        .name("hermes desktop fs daemon".to_string())
        .spawn(move || daemon.run())
        .expect("could not launch fs daemon");

    let event_rx = fs_event::FsDaemonEventReceiver::new(event_rx);
    app.manage(event_rx.clone());
    app.manage(fs_event::FsDaemonCommandSender::new(command_tx));
    tauri::async_runtime::spawn(fs_event::handle_events(app.handle().clone()));
    Ok(())
}

mod utils {
    use hermes_desktop_lib as lib;
    use std::path::PathBuf;

    pub async fn load_dataset(
        path: impl Into<PathBuf>,
    ) -> Result<lib::data::Dataset, lib::data::error::Load> {
        use lib::data::Dataset;

        let path = path.into();
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
            FileKind::Csv => tauri::async_runtime::spawn_blocking({
                move || lib::data::Csv::load_from_path(&path)
            })
            .await
            .expect("tauri async runtime failed")
            .map(|csv| csv.into())
            .map_err(|err| err.into()),

            FileKind::Excel => tauri::async_runtime::spawn_blocking({
                move || lib::data::Workbook::load_from_path(&path)
            })
            .await
            .expect("tauri async runtime failed")
            .map(|workbook| workbook.into())
            .map_err(|err| err.into()),

            FileKind::Unknown => {
                match tauri::async_runtime::spawn_blocking({
                    let path = path.clone();
                    move || lib::data::Csv::load_from_path(&path)
                })
                .await
                .expect("tauri async runtime failed")
                {
                    Ok(csv) => Ok(csv.into()),
                    Err(csv_err) => match csv_err {
                        lib::data::error::LoadCsv::Io(_) => Err(csv_err.into()),
                        _ => match tauri::async_runtime::spawn_blocking({
                            let path = path.clone();
                            move || lib::data::Workbook::load_from_path(&path)
                        })
                        .await
                        .expect("tauri async runtime failed")
                        {
                            Ok(workbook) => Ok(workbook.into()),
                            Err(_) => Err(lib::data::error::Load::InvalidFileType),
                        },
                    },
                }
            }
        }
    }

    #[derive(Debug)]
    enum FileKind {
        Csv,
        Excel,
        Unknown,
    }
}

mod commands {
    use crate::{fs_event, utils};
    use hermes_core as core;
    use hermes_desktop_lib as lib;
    use hermes_fs_daemon as fs_daemon;
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
    pub async fn load_directory(
        fs_command_tx: tauri::State<'_, fs_event::FsDaemonCommandSender>,
        root: PathBuf,
    ) -> Result<lib::fs::DirectoryTree, lib::fs::error::FromFileSystem> {
        let res = lib::fs::DirectoryTree::from_file_system(&root);
        if res.is_ok() {
            fs_command_tx
                .lock()
                .await
                .send(fs_daemon::server::Command::Watch(root))
                .unwrap();
        }
        res
    }

    #[tauri::command]
    pub async fn load_dataset(path: PathBuf) -> Result<lib::data::Dataset, lib::data::error::Load> {
        utils::load_dataset(&path).await
    }

    /// Run workspace orders.
    ///
    /// # Returns
    /// If errors occur, returns a `Vec<(<order index>, <error>)>`.
    #[tauri::command]
    pub async fn run_workspace(
        orders: Vec<lib::formula::WorkspaceOrder>,
    ) -> Result<(), Vec<lib::formula::error::WorkspaceOrder>> {
        let formulas = orders
            .iter()
            .flat_map(|order| match order {
                lib::formula::WorkspaceOrder::Create => todo!(),
                lib::formula::WorkspaceOrder::Update(update) => update.formulas(),
            })
            .collect::<Vec<_>>();
        let mut tasks = tokio::task::JoinSet::new();
        let mut task_handles = Vec::with_capacity(orders.len());
        for order in orders {
            let handle = tasks.spawn(run_workspace_order(order));
            task_handles.push(handle);
        }

        let mut errors = Vec::new();
        while let Some(result) = tasks.join_next().await {
            match result {
                Ok(result) => {
                    if let Err(err) = result {
                        errors.push(err)
                    }
                }

                Err(err) => {
                    let err = lib::formula::error::WorkspaceOrder::new(
                        formulas.clone(),
                        lib::formula::error::WorkspaceOrderKind::TaskNotCompleted,
                    );
                    errors.push(err);
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
        use core::expr::Value;

        #[cfg(feature = "tracing")]
        tracing::trace!("processing orders");

        let formulas = updates
            .iter()
            .map(|update| update.formula().clone())
            .collect::<Vec<_>>();

        let file = tokio::fs::File::open(&path)
            .await
            .map_err(|err| {
                let err = lib::formula::error::WorkspaceOrderKind::OpenFile {
                    path: path.clone(),
                    error: err.kind(),
                };

                lib::formula::error::WorkspaceOrder::new(formulas.clone(), err)
            })?
            .into_std()
            .await;

        let mut csv = lib::data::Csv::from_reader(file).map_err(|err| {
            let err = match err {
                lib::data::error::LoadCsv::Io(err) => err,
                lib::data::error::LoadCsv::DataTooLarge => todo!(),
            };
            let err = lib::formula::error::WorkspaceOrderKind::OpenFile {
                path: path.clone(),
                error: err,
            };
            lib::formula::error::WorkspaceOrder::new(formulas.clone(), err)
        })?;

        for update in updates {
            let (formula, row, col, value) = update.into_parts();
            let idx = core::data::CellIndex::new(row, col);
            if let Some(sheet_value) = csv.sheet.get(&idx) {
                let empty_value = match sheet_value {
                    Value::Empty => true,
                    Value::String(value) => value.is_empty(),
                    Value::Int(_) => false,
                    Value::Float(_) => false,
                    Value::Bool(_) => false,
                    Value::DateTime(date_time) => false,
                    Value::Duration(duration) => false,
                };

                if empty_value {
                    csv.sheet.set(idx.clone(), value.clone());
                } else {
                    #[cfg(feature = "tracing")]
                    tracing::debug!(?idx, ?value);
                    todo!();
                }
            } else {
                csv.sheet.insert(idx, value).expect("cell should be empty");
            }
        }

        csv.save(&path).map_err(|err| {
            let err = match err {
                lib::data::error::SaveCsv::Io(err) => err,
            };
            let err = lib::formula::error::WorkspaceOrderKind::Save { path, error: err };
            lib::formula::error::WorkspaceOrder::new(formulas.clone(), err)
        })?;
        Ok(())
    }

    async fn run_workspace_order_update_workbook(
        path: PathBuf,
        updates: Vec<lib::formula::UpdateWorkbook>,
    ) -> Result<(), lib::formula::error::WorkspaceOrder> {
        todo!();
    }
}
