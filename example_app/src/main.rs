#![forbid(unsafe_code)]
#![warn(clippy::all, rust_2018_idioms)]
#![allow(clippy::manual_range_contains)]

#[tokio::main]
async fn main() {
    let remote_logger = native_helper::RemoteLogger::new().await;

    {
        let _guard = tracing::info_span!("main").entered();

        for run in 0..2 {
            let _guard = tracing::info_span!("run", run).entered();

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

            std::thread::sleep(std::time::Duration::from_millis(20));

            {
                let _guard = tracing::info_span!("join").entered();
                for handle in handles {
                    handle.await.unwrap();
                }
            }
        }
    }

    remote_logger.join().await;
}

#[tracing::instrument]
pub fn my_function() {
    let span = tracing::info_span!("my_span");
    span.in_scope(|| {
        tracing::info!("Hello from my_function");
        tracing::event!(tracing::Level::INFO, value = 42_i32, "This is an event");
        std::thread::sleep(std::time::Duration::from_millis(5));
    });
    std::thread::sleep(std::time::Duration::from_millis(5));
    span.in_scope(|| {
        tracing::info!("Second time in same span");
        std::thread::sleep(std::time::Duration::from_millis(5));
    });
}
