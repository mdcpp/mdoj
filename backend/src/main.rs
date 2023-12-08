pub mod controller;
pub mod endpoint;
pub mod grpc;
pub mod init;
pub mod macro_tool;
pub mod server;

#[tokio::main]
async fn main() {
    let server = server::Server::new().await;
    server.start().await;
}
