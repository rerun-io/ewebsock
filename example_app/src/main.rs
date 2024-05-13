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
// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main_impl() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let app = example_app::ExampleApp::default();
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "ewebsocket example app",
        native_options,
        Box::new(|_cc| Box::new(app)),
    )
}

// When compiling to web using trunk:
#[cfg(target_arch = "wasm32")]
fn main_impl() -> Result<(), eframe::Error> {
    // Redirect `log` message to `console.log` and friends:
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        eframe::WebRunner::new()
            .start(
                "the_canvas_id", // hardcode it
                web_options,
                Box::new(|_cc| Box::new(example_app::ExampleApp::default())),
            )
            .await
            .expect("failed to start eframe");
    });
    Ok(())
}
