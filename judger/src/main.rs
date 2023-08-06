use crate::grpc::server::GRpcServer;
use grpc::proto::prelude::judger_server::JudgerServer;
use init::config::CONFIG;
use tonic::transport::Server;

// use crate::plugin::proto::prelude::judge_service_server::JudgeServiceServer;

pub mod grpc;
pub mod init;
pub mod sandbox;
pub mod langs;

#[tokio::main]
async fn main() {
    init::new().await;

    let config = CONFIG.get().unwrap();
    let addr = config.runtime.bind.parse().unwrap();

    log::info!("Server started");

    let server = GRpcServer::new().await;

    Server::builder()
        .add_service(JudgerServer::new(server))
        .serve(addr)
        .await
        .unwrap();
}
