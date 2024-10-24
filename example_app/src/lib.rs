//! Example application.

mod app;
pub use app::ExampleApp;

#[cfg(target_arch = "wasm32")]
mod web;
