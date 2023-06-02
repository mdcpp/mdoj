use init::config::CONFIG;
use tonic::transport::Server;

pub mod init;
pub mod jail;
pub mod plugin;

#[tokio::main]
async fn main() {
    init::new().await;

    let config = CONFIG.get().unwrap();
    let addr = config.runtime.bind.parse().unwrap();

    let plugin_provider = plugin::plugin::LangJudger::new().await;

    log::info!("Server started");
    Server::builder()
        .add_service(
            plugin::proto::prelude::plugin_provider_server::PluginProviderServer::new(
                plugin_provider,
            ),
        )
        .serve(addr)
        .await
        .unwrap();
}
