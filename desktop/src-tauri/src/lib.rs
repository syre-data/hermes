#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::select_folder,
            commands::load_directory,
            commands::load_workbook,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

mod commands {
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
    pub fn load_workbook(path: PathBuf) -> Result<lib::data::Workbook, lib::data::error::Load> {
        use lib::data::Workbook;

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
            FileKind::Csv => Workbook::load_csv(&path).map_err(|err| err.into()),
            FileKind::Excel => Workbook::load_excel(&path).map_err(|err| err.into()),
            FileKind::Unknown => match Workbook::load_csv(&path) {
                Ok(workbook) => Ok(workbook),
                Err(csv_err) => match csv_err {
                    lib::data::error::LoadCsv::Io(_) => Err(csv_err.into()),
                    _ => match Workbook::load_excel(&path) {
                        Ok(workbook) => Ok(workbook),
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
}
