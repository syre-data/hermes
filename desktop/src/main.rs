use leptos::prelude::*;

fn main() {
    #[cfg(all(feature = "tracing", debug_assertions))]
    tracing::enable();
    console_error_panic_hook::set_once();
    mount_to_body(hermes_ui::App);
}

#[cfg(all(feature = "tracing", debug_assertions))]
mod tracing {
    use tracing_subscriber::{filter, fmt::time::UtcTime, prelude::*};

    pub fn enable() {
        let target_filter = filter::Targets::new()
            .with_target("hermes", tracing::Level::TRACE)
            .with_target("leptos", tracing::Level::TRACE);
        let fmt_layer = tracing_subscriber::fmt::layer()
            .with_ansi(false) // Only partially supported across browsers
            // .with_timer(UtcTime::rfc_3339())
            .without_time()
            .pretty()
            .with_writer(tracing_web::MakeWebConsoleWriter::new()); // write events to the console

        let layers = tracing_subscriber::registry().with(fmt_layer);
        layers.with(target_filter).init();
    }
}
