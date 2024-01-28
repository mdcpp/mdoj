use std::sync::Arc;

use grpc::prelude::judger_server::JudgerServer;
use init::config::CONFIG;

pub mod grpc;
pub mod init;
pub mod langs;
pub mod sandbox;
pub mod server;
#[cfg(test)]
pub mod tests;

#[tokio::main]
async fn main() {
    init::new().await;

    let config = CONFIG.get().unwrap();
    let addr = config.runtime.bind.parse().unwrap();

    log::info!("Server started");

    let server = server::Server::new().await;

    tonic::transport::Server::builder()
        .add_service(JudgerServer::new(Arc::new(server)))
        .serve(addr)
        .await
        .unwrap();
}
