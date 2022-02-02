#![forbid(unsafe_code)]
#![warn(clippy::all, rust_2018_idioms)]

#[cfg(not(target_arch = "wasm32"))]
#[cfg(feature = "websocket")]
pub mod native_websocket;

#[cfg(not(target_arch = "wasm32"))]
#[cfg(feature = "with_tungstenite")]
pub mod native_tungstenite;

#[cfg(not(target_arch = "wasm32"))]
#[cfg(feature = "with_tungstenite")]
pub use native_tungstenite::*;

#[cfg(not(target_arch = "wasm32"))]
#[cfg(feature = "websocket")]
#[cfg(not(feature = "with_tungstenite"))]
pub use native_websocket::*;

#[cfg(target_arch = "wasm32")]
mod web;

#[cfg(target_arch = "wasm32")]
pub use web::*;

// ----------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub enum WsMessage {
    Binary(Vec<u8>),
    Text(String),
    Unknown(String),

    /// Only for native
    Ping(Vec<u8>),
    /// Only for native
    Pong(Vec<u8>),
}

#[derive(Clone, Debug)]
pub enum WsEvent {
    Message(WsMessage),
    Error(String),
    Opened,
    Closed,
}

pub struct WsReceiver {
    rx: std::sync::mpsc::Receiver<WsEvent>,
}

impl WsReceiver {
    pub fn new(wake_up: impl Fn() + Send + Sync + 'static) -> (Self, EventHandler) {
        let (tx, rx) = std::sync::mpsc::channel();
        let tx = std::sync::Mutex::new(tx);
        let on_event = std::sync::Arc::new(move |event| {
            wake_up(); // wake up UI thread
            if tx.lock().unwrap().send(event).is_ok() {
                std::ops::ControlFlow::Continue(())
            } else {
                std::ops::ControlFlow::Break(())
            }
        });
        let ws_receiver = WsReceiver { rx };
        (ws_receiver, on_event)
    }

    pub fn try_recv(&self) -> Option<WsEvent> {
        self.rx.try_recv().ok()
    }
}

pub type Error = String;
pub type Result<T> = std::result::Result<T, Error>;

pub type EventHandler = std::sync::Arc<dyn Sync + Send + Fn(WsEvent) -> std::ops::ControlFlow<()>>;
