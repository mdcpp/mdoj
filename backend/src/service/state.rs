use std::sync::Arc;

use crate::{proto::prelude::{plugin_provider_client::PluginProviderClient}, init::config::CONFIG};
use sea_orm::{DatabaseConnection, Database};
use tonic::transport::Channel;

struct ServiceState {
    db: DatabaseConnection,
    sandbox: PluginProviderClient<Channel>,
}

impl ServiceState {
    async fn new() -> Arc<ServiceState> {
        let config=CONFIG.get().unwrap();

        let db=Database::connect(&config.database.uri).await.unwrap();

        let sandbox = PluginProviderClient::connect("http://127.0.0.1:8080")
            .await
            .unwrap();
        Arc::new(ServiceState {
            db,
            sandbox,
        })
    }
}
