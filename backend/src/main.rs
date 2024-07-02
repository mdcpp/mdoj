pub mod controller;
pub mod endpoint;
pub mod entity;
pub mod init;
pub mod macro_tool;
pub mod server;
pub mod util;

#[cfg(feature = "release")]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

pub type TonicStream<T> =
    std::pin::Pin<Box<dyn tokio_stream::Stream<Item = Result<T, tonic::Status>> + Send>>;

#[tokio::main]
async fn main() {
    let server = server::Server::new().await.unwrap();
    server.start().await;
}
