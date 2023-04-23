#![forbid(unsafe_code)]
#![cfg_attr(not(debug_assertions), deny(warnings))] // Forbid warnings in release builds
#![warn(clippy::all, rust_2018_idioms)]

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "ewebsocket example app",
        native_options,
        Box::new(|cc| Box::new(example_app::ExampleApp::new(cc))),
    )
    .expect("failed to start eframe");
}

// when compiling to web using trunk.
#[cfg(target_arch = "wasm32")]
fn main() {
    console_error_panic_hook::set_once();
    tracing_wasm::set_as_global_default();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        eframe::start_web(
            "ewebsocket example app",
            web_options,
            Box::new(|cc| Box::new(example_app::ExampleApp::new(cc))),
        )
        .await;
    });
}
