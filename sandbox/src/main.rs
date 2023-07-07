use init::config::CONFIG;
use tonic::transport::Server;

use crate::plugin::proto::prelude::judge_service_server::JudgeServiceServer;

pub mod init;
pub mod limit;
pub mod plugin;

#[tokio::main]
async fn main() {
    init::new().await;

    let config = CONFIG.get().unwrap();
    let addr = config.runtime.bind.parse().unwrap();

    let plugin_provider = plugin::plugin::LangJudger::new().await;

    log::info!("Server started");
    Server::builder()
        .add_service(JudgeServiceServer::new(plugin_provider))
        .serve(addr)
        .await
        .unwrap();
}
