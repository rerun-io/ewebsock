#![forbid(unsafe_code)]
#![cfg_attr(not(debug_assertions), deny(warnings))] // Forbid warnings in release builds
#![warn(clippy::all, rust_2018_idioms)]

#[cfg(feature = "tokio")]
#[tokio::main]
async fn main() -> eframe::Result<()> {
    main_impl()
}

#[cfg(not(feature = "tokio"))]
fn main() -> eframe::Result<()> {
    main_impl()
}

fn main_impl() -> Result<(), eframe::Error> {
    env_logger::init();
    // Log to stderr (if you run with `RUST_LOG=debug`).

    let app = example_app::ExampleApp::default();
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "ewebsocket example app",
        native_options,
        Box::new(|_cc| Box::new(app)),
    )
}
