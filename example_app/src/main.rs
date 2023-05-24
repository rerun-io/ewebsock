#![forbid(unsafe_code)]
#![cfg_attr(not(debug_assertions), deny(warnings))] // Forbid warnings in release builds
#![warn(clippy::all, rust_2018_idioms)]

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() -> eframe::Result<()> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let app = example_app::ExampleApp::default();
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "ewebsocket example app",
        native_options,
        Box::new(|_cc| Box::new(app)),
    )
}
