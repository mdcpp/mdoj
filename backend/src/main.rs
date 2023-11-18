pub mod controller;
pub mod endpoint;
pub mod grpc;
pub mod init;
pub mod server;

#[tokio::main]
async fn main() {
    init::new().await;
    server::Server::start().await;
}
