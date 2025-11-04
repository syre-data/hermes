//! Runs an fs daemon.
//!
//! Must be run with the `server` feature enabled.

fn main() {
    #[cfg(feature = "server")]
    server::main();
    #[cfg(not(feature = "server"))]
    panic!("must be run with `server` feature enabled.");
}

#[cfg(feature = "server")]
mod server {
    use std::{io::Write, path::PathBuf};

    use hermes_fs_daemon::server;

    /// Run the database with the default config.
    ///
    /// # Notes
    /// + Must run with the `server` feature enabled.
    pub fn main() {
        #[cfg(feature = "tracing")]
        logging::enable();

        let default_panic_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic_info| {
            panic_hook(panic_info);
            default_panic_hook(panic_info);
        }));

        let (command_tx, command_rx) = server::command_channel();
        let (event_tx, event_rx) = server::event_channel();
        let mut daemon = server::Daemon::new(event_tx, command_rx);

        let daemon_handle = std::thread::Builder::new()
            .name("daemon".to_string())
            .spawn(move || daemon.run())
            .expect("could not launch daemon");

        let mut user_in = String::new();
        loop {
            std::io::stdin().read_line(&mut user_in).unwrap();
            let Some(cmd) = parse_user_input_to_command(&user_in) else {
                std::io::stdout().write_all(b"invalid command").unwrap();
                continue;
            };
            command_tx.send(cmd).unwrap();
        }
    }

    fn parse_user_input_to_command(input: &String) -> Option<server::Command> {
        let mut ins = input.split_ascii_whitespace();
        let Some(cmd) = ins.next() else {
            return None;
        };
        match cmd {
            "watch" => {
                let Some(path) = ins.next() else {
                    return None;
                };
                Some(server::Command::Watch(PathBuf::from(path)))
            }
            "unwatch" => {
                let Some(path) = ins.next() else {
                    return None;
                };
                Some(server::Command::Unwatch(PathBuf::from(path)))
            }
            _ => None,
        }
    }

    fn panic_hook(panic_info: &std::panic::PanicHookInfo) {
        let payload = if let Some(payload) = panic_info.payload().downcast_ref::<&str>() {
            Some(&**payload)
        } else if let Some(payload) = panic_info.payload().downcast_ref::<String>() {
            Some(payload.as_str())
        } else {
            None
        };

        let location = panic_info.location().map(|location| location.to_string());
        #[cfg(feature = "tracing")]
        tracing::error!("fs daemon panicked at {location:?}: {payload:?}");
        // TODO: If `tracing` is nor enabled?
    }

    #[cfg(feature = "tracing")]
    mod logging {
        use std::io;
        use tracing_subscriber::{
            EnvFilter, Registry,
            fmt::{self, time::UtcTime},
            prelude::*,
        };

        /// Enable logging.
        pub fn enable() {
            let console_logger = fmt::layer()
                .with_writer(io::stdout)
                .with_timer(UtcTime::rfc_3339())
                .pretty();

            let subscriber = Registry::default()
                .with(EnvFilter::from_default_env())
                .with(console_logger);

            tracing::subscriber::set_global_default(subscriber).unwrap();
        }
    }
}
