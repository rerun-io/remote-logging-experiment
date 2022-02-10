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
        let _guard = tracing::info_span!("main").entered();

        let mut handles = vec![];
        {
            let _guard = tracing::info_span!("spawn").entered();

            for task_nr in 0..2 {
                let child_span = tracing::info_span!("task", task_nr).or_current();
                let handle = tokio::task::spawn_blocking(move || {
                    child_span.in_scope(|| {
                        my_function();
                    });
                });
                handles.push(handle);
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(10));

        for handle in handles {
            handle.await.unwrap();
        }
    }

    std::thread::sleep(std::time::Duration::from_millis(100)); // give time to send it
}

#[tracing::instrument]
pub fn my_function() {
    let span = tracing::info_span!("my_span");
    span.in_scope(|| {
        tracing::info!("Hello from my_function");
        tracing::event!(tracing::Level::INFO, value = 42_i32, "This is an event");
        std::thread::sleep(std::time::Duration::from_millis(10));
    });
    span.in_scope(|| {
        tracing::info!("Second time in same span");
        std::thread::sleep(std::time::Duration::from_millis(10));
    });
}
