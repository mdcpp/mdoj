pub mod controller;
pub mod endpoint;
pub mod entity;
pub mod grpc;
pub mod init;
pub mod macro_tool;
pub mod server;
pub mod util;

#[cfg(features = "release")]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

#[tokio::main]
async fn main() {
    let server = server::Server::new().await.unwrap();
    server.start().await;
}
