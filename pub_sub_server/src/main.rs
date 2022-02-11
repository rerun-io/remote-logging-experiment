#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    pub_sub_server::run("127.0.0.1:9002").await.unwrap();
}
