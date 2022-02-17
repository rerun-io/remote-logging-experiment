#![forbid(unsafe_code)]
#![warn(clippy::all, rust_2018_idioms)]
#![allow(clippy::manual_range_contains)]

/// Helper that starts all required services locally and opens an URL with the viewer.
///
/// For when you don't want to use external servers.
pub struct RemoteLogger {
    join_handles: Vec<tokio::task::JoinHandle<()>>,
}

impl RemoteLogger {
    /// Starts up the necessary servers and installs [`tracing`] log hooks.
    pub async fn new() -> Self {
        let mut join_handles = vec![];

        let pub_sub_port = rr_data::DEFAULT_PUB_SUB_PORT;
        let pub_sub_url = format!("ws://127.0.0.1:{}", rr_data::DEFAULT_PUB_SUB_PORT);

        #[cfg(feature = "pub_sub_server")]
        {
            eprintln!("Starting pub-sub-server…");
            let server = pub_sub_server::Server::new(pub_sub_port).await.unwrap();
            join_handles.push(tokio::spawn(async move {
                server.run().await.unwrap();
            }));
        };

        logger::setup_logging(&pub_sub_url); // This starts sending things to pub-sub server

        #[cfg(feature = "web_server")]
        {
            tracing::debug!("Starting web server…");
            let port = rr_data::DEFAULT_VIEWER_WEB_SERVER_PORT;
            join_handles.push(tokio::spawn(async move {
                web_server::run(port).await.unwrap();
            }));

            #[cfg(feature = "webbrowser")]
            {
                std::thread::sleep(std::time::Duration::from_millis(100)); // give web server time to start
                let viewer_url = format!("http://127.0.0.1:{}?pubsub={}", port, pub_sub_url);
                webbrowser::open(&viewer_url).ok();
            }
        };

        Self { join_handles }
    }

    /// Waits for servers to shut down (on SIGINT).
    pub async fn join(mut self) {
        for handle in self.join_handles.drain(..) {
            handle.await.unwrap();
        }
    }
}
