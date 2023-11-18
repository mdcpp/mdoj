pub mod controller;
pub mod endpoint;
pub mod grpc;
pub mod init;
pub mod server;

#[tokio::main]
async fn main() {
    server::Server::start().await;
}
