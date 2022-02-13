#[tokio::main]
async fn main() {
    let pub_sub_port = rr_data::DEFAULT_PUB_SUB_PORT;
    logger::setup_logging(&format!("ws://127.0.0.1:{}", pub_sub_port));
    tracing::debug!("Loggin set up");

    #[cfg(feature = "pub_sub_server")]
    let pub_sub_handle = tokio::spawn(async move {
        tracing::debug!("Starting pub-sub-server…");
        pub_sub_server::run(pub_sub_port).await.unwrap();
    });

    #[cfg(feature = "web_server")]
    let web_server_handle = {
        tracing::debug!("Starting web server…");
        let port = rr_data::DEFAULT_VIEWER_WEB_SERVER_PORT;
        let web_server_handle = tokio::spawn(async move {
            web_server::run(port).await.unwrap();
        });
        #[cfg(feature = "webbrowser")]
        {
            let url = format!("http://127.0.0.1:{}", port);
            if webbrowser::open(&url).is_ok() {
                std::thread::sleep(std::time::Duration::from_millis(1000)); // give it time to start
            }
        }
        web_server_handle
    };

    std::thread::sleep(std::time::Duration::from_millis(100)); // give everything time to start

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

    #[cfg(feature = "pub_sub_server")]
    pub_sub_handle.await.unwrap();
    #[cfg(feature = "web_server")]
    web_server_handle.await.unwrap();
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
