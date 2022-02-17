#![forbid(unsafe_code)]
#![warn(clippy::all, rust_2018_idioms)]
#![allow(clippy::manual_range_contains)]

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let port = rr_data::DEFAULT_VIEWER_WEB_SERVER_PORT;
    web_server::run(port).await.unwrap();
}
