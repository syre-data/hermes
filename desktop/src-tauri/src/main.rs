// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    #[cfg(feature = "tracing")]
    let _log_guard = logging::enable();
    hermes_lib::run()
}

#[cfg(feature = "tracing")]
mod logging {
    use tracing_subscriber::{
        EnvFilter, Registry, filter,
        fmt::{self, time},
        prelude::*,
    };

    const LOG_DIR: &str = ".";
    const LOG_PREFIX: &str = "hermes.log";

    #[cfg(debug_assertions)]
    const HERMES_LOG_LEVEL_FILE: tracing::Level = tracing::Level::DEBUG;
    #[cfg(not(debug_assertions))]
    const HERMES_LOG_LEVEL_FILE: tracing::Level = tracing::Level::ERROR;

    pub fn enable() -> tracing_appender::non_blocking::WorkerGuard {
        let file_filter = filter::Targets::default()
            .with_default(tracing::Level::ERROR)
            .with_target("hermes", HERMES_LOG_LEVEL_FILE);

        // TODO: Setup log directory
        let file_logger = tracing_appender::rolling::daily(LOG_DIR, LOG_PREFIX);
        let (file_logger, _log_guard) = tracing_appender::non_blocking(file_logger);
        let file_logger = fmt::layer()
            .with_writer(file_logger)
            .with_timer(time::UtcTime::rfc_3339())
            .json()
            .with_filter(file_filter);

        #[cfg(debug_assertions)]
        let console_logger = fmt::layer()
            .with_writer(std::io::stdout)
            .with_timer(time::UtcTime::rfc_3339())
            .pretty();

        let subscriber = Registry::default()
            .with(EnvFilter::from_default_env())
            .with(file_logger);

        #[cfg(debug_assertions)]
        let subscriber = subscriber.with(console_logger);

        tracing::subscriber::set_global_default(subscriber).unwrap();
        _log_guard
    }
}
