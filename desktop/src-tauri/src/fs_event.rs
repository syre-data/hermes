use crate::utils;
use hermes_desktop_lib::event;
use hermes_fs_daemon as fs_daemon;
use std::sync::Arc;
use tauri::{Emitter, Manager};

#[derive(derive_more::Deref, Clone)]
pub struct FsDaemonEventReceiver(Arc<tokio::sync::Mutex<fs_daemon::server::EventReceiver>>);
impl FsDaemonEventReceiver {
    pub fn new(event_rx: fs_daemon::server::EventReceiver) -> Self {
        Self(Arc::new(tokio::sync::Mutex::new(event_rx)))
    }
}

#[derive(derive_more::Deref, Clone)]
pub struct FsDaemonCommandSender(Arc<tokio::sync::Mutex<fs_daemon::server::CommandSender>>);
impl FsDaemonCommandSender {
    pub fn new(command_tx: fs_daemon::server::CommandSender) -> Self {
        Self(Arc::new(tokio::sync::Mutex::new(command_tx)))
    }
}

#[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
pub async fn handle_events(app: tauri::AppHandle) {
    let event_rx = app.state::<FsDaemonEventReceiver>();
    while let Some(fs_events) = event_rx.lock().await.recv().await {
        #[cfg(feature = "tracing")]
        tracing::trace!(?fs_events);

        let mut tasks = tokio::task::JoinSet::new();
        for fs_event in fs_events {
            tasks.spawn(process_event(fs_event));
        }
        let app_events = tasks
            .join_all()
            .await
            .into_iter()
            .flat_map(|result| match result {
                Ok(events) => events
                    .into_iter()
                    .map(|event| Ok(event))
                    .collect::<Vec<_>>(),
                Err(err) => vec![Err(err)],
            })
            .collect::<Vec<_>>();

        #[cfg(feature = "tracing")]
        tracing::trace!(?app_events);
        let w_main = app
            .get_webview_window("main")
            .expect("main webview to exist");
        w_main
            .emit(event::EVENT_TOPIC, app_events)
            .expect("emit events to succeed");
    }
}

async fn process_event(event: fs_daemon::Event) -> Result<Vec<event::Event>, event::Error> {
    match event {
        fs_daemon::Event::File(_) => process_event_file(&event).await,
        fs_daemon::Event::Folder(_) => process_event_folder(&event).await,
        fs_daemon::Event::Any(_) => process_event_any(&event).await,
    }
}

#[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
async fn process_event_file(event: &fs_daemon::Event) -> Result<Vec<event::Event>, event::Error> {
    let fs_daemon::Event::File(kind) = event else {
        panic!("invalid event kind");
    };

    match kind {
        fs_daemon::event::File::Created(_) => todo!(),
        fs_daemon::event::File::Created(_) => todo!(),
        fs_daemon::event::File::Removed(_) => todo!(),
        fs_daemon::event::File::Renamed { .. } => todo!(),
        fs_daemon::event::File::Moved { .. } => todo!(),
        fs_daemon::event::File::Modified(_) => process_event_file_modified(event).await,
    }
}

#[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
async fn process_event_file_modified(
    event: &fs_daemon::Event,
) -> Result<Vec<event::Event>, event::Error> {
    let fs_daemon::Event::File(fs_daemon::event::File::Modified(path)) = event else {
        panic!("invalid event kind");
    };

    utils::load_dataset(path)
        .await
        .map(|dataset| {
            let event = event::File::Modified {
                path: path.clone(),
                data: dataset,
            };

            vec![event.into()]
        })
        .map_err(|error| event::Error::LoadDataset {
            path: path.clone(),
            error,
        })
}

#[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
async fn process_event_folder(event: &fs_daemon::Event) -> Result<Vec<event::Event>, event::Error> {
    let fs_daemon::Event::Folder(event) = event else {
        panic!("invalid event kind");
    };

    match event {
        fs_daemon::event::Folder::Created(_) => todo!(),
        fs_daemon::event::Folder::Removed(_) => todo!(),
        fs_daemon::event::Folder::Renamed { .. } => todo!(),
        fs_daemon::event::Folder::Moved { .. } => todo!(),
    }
}

#[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
async fn process_event_any(event: &fs_daemon::Event) -> Result<Vec<event::Event>, event::Error> {
    let fs_daemon::Event::Any(event) = event else {
        panic!("invalid event kind");
    };

    match event {
        hermes_fs_daemon::event::Any::Removed(_) => todo!(),
    }
}
