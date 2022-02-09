fn setup_logging() {
    use tracing_subscriber::prelude::*;

    let stdout_logger = tracing_subscriber::fmt::layer();
    let stdout_logger =
        stdout_logger.with_filter(tracing_subscriber::filter::filter_fn(|metadata| {
            metadata.level() <= &tracing::Level::DEBUG
        }));

    let url = "ws://127.0.0.1:9002";
    let topic_meta = rr_data::TopicMeta {
        created: rr_data::Time::now(),
        name: "logger".into(),
    };
    let rr_logger = logger::RrLogger::to_ws_server(url.into(), topic_meta);
    let rr_logger = rr_logger.with_filter(tracing_subscriber::filter::filter_fn(|metadata| {
        metadata.level() <= &tracing::Level::INFO
    }));

    tracing_subscriber::registry()
        .with(stdout_logger)
        .with(rr_logger)
        .init();
}

#[tokio::main]
async fn main() {
    setup_logging();
    std::thread::sleep(std::time::Duration::from_millis(100)); // give it time to start

    {
        let main_span = tracing::info_span!("main");
        let _guard = main_span.entered();

        // is there a simpler way to do this?
        let child_span = tracing::info_span!(parent: None, "task0");
        child_span.follows_from(tracing::Span::current());
        let handle_0 = tokio::task::spawn_blocking(move || {
            child_span.in_scope(|| {
                my_function();
            });
        });

        let child_span = tracing::info_span!(parent: None, "task1");
        child_span.follows_from(tracing::Span::current());
        let handle_1 = tokio::task::spawn_blocking(move || {
            child_span.in_scope(|| {
                my_function();
            });
        });

        handle_0.await.unwrap();
        handle_1.await.unwrap();
    }

    std::thread::sleep(std::time::Duration::from_millis(100)); // give time to send it all away
}

#[tracing::instrument]
pub fn my_function() {
    let span = tracing::info_span!("my_span");
    span.in_scope(|| {
        tracing::info!("Hello from my_function");
        tracing::event!(tracing::Level::INFO, value = 42_i32, "This is an event");
        std::thread::sleep(std::time::Duration::from_millis(50));
    });
    span.in_scope(|| {
        tracing::info!("Second time in same span");
        std::thread::sleep(std::time::Duration::from_millis(50));
    });
}
