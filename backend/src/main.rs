mod config;
mod controller;
mod endpoint;
mod entity;
mod macro_tool;
mod server;
mod util;

// FIXME: replace relase feature with debug_assertions
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
