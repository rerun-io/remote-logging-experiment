#![forbid(unsafe_code)]
#![warn(clippy::all, rust_2018_idioms)]

#[cfg(not(target_arch = "wasm32"))]
mod native;

#[cfg(not(target_arch = "wasm32"))]
pub use native::*;

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
}

type Error = String;
type Result<T> = std::result::Result<T, Error>;
