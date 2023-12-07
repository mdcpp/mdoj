pub mod controller;
pub mod endpoint;
pub mod grpc;
pub mod init;
pub mod macro_tool;
pub mod server;

#[tokio::main]
async fn main() {
    init::new().await;
    log::info!("starting server");
    server::Server::start().await;
}
 