#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let port = rr_data::DEFAULT_PUB_SUB_PORT;
    pub_sub_server::run(port).await.unwrap();
}
