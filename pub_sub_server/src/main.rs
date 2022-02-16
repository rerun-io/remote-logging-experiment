#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let port = rr_data::DEFAULT_PUB_SUB_PORT;
    let server = pub_sub_server::Server::new(port).await.unwrap();
    server.run().await.unwrap();
}
