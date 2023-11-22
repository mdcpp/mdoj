use crate::grpc::server::Server;
use grpc::proto::prelude::judger_server::JudgerServer;
use init::config::CONFIG;
use tonic::transport;
// use crate::plugin::proto::prelude::judge_service_server::JudgeServiceServer;

pub mod grpc;
pub mod init;
pub mod langs;
pub mod sandbox;
pub mod test;

#[tokio::main]
async fn main() {
    init::new().await;

    let config =    CONFIG.get().unwrap()   ;
    let addr = config.runtime.bind.parse().unwrap();

    log::info!("Server started");

    let server = Server::new().await;

    transport::Server::builder()
        .add_service(JudgerServer::new(server))
        .serve(addr)
        .await
        .unwrap();
}
