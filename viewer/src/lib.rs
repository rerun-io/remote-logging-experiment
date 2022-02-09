#![forbid(unsafe_code)]
#![warn(clippy::all, rust_2018_idioms)]
#![allow(clippy::manual_range_contains)]

// ----------------------------------------------------------------------------

mod app;
mod event_log;
mod misc;
mod span_tree;
mod viewer;
pub use app::WsClientApp;

// ----------------------------------------------------------------------------
// When compiling for web:

#[cfg(target_arch = "wasm32")]
use eframe::wasm_bindgen::{self, prelude::*};

/// This is the entry-point for all the web-assembly.
/// This is called once from the HTML.
/// It loads the app, installs some callbacks, then returns.
/// You can add more callbacks like this if you want to call in to your code.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn start(canvas_id: &str) -> std::result::Result<(), eframe::wasm_bindgen::JsValue> {
    let app = WsClientApp::default();
    eframe::start_web(canvas_id, Box::new(app))
}
