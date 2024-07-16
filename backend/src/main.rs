pub mod config;
pub mod controller;
pub mod endpoint;
pub mod entity;
pub mod macro_tool;
pub mod server;
pub mod util;

// FIXME: replace relase feature with debug_assertions
#[cfg(feature = "release")]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

pub type TonicStream<T> =
    std::pin::Pin<Box<dyn tokio_stream::Stream<Item = Result<T, tonic::Status>> + Send>>;

#[tokio::main]
async fn main() {
    server::OtelGuard::new()
        .unwrap()
        .with(async {
            let server = server::Server::new().await.unwrap();
            server.start().await;
        })
        .await;
}
