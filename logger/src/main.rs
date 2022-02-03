#[tokio::main]
async fn main() {
    let url = "ws://127.0.0.1:9002";

    let rr_logger = logger::RrLogger::to_ws_server(url.into());

    use tracing_subscriber::prelude::*;

    let stdout_logger = tracing_subscriber::fmt::layer();
    let stdout_logger =
        stdout_logger.with_filter(tracing_subscriber::filter::filter_fn(|metadata| {
            metadata.level() <= &tracing::Level::DEBUG
        }));

    let rr_logger = rr_logger.with_filter(tracing_subscriber::filter::filter_fn(|metadata| {
        metadata.level() <= &tracing::Level::INFO
    }));

    tracing_subscriber::registry()
        .with(stdout_logger)
        .with(rr_logger)
        .init();

    my_function();
    my_function();

    std::thread::sleep(std::time::Duration::from_secs(1));
}

#[tracing::instrument]
pub fn my_function() {
    tracing::info!("Hello from my_function");
    tracing::event!(tracing::Level::INFO, value = 42_i32, "This is an event");
}
