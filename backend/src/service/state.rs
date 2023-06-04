use std::sync::Arc;

use crate::{proto::prelude::judge_service_client::JudgeServiceClient, init::config::CONFIG};
use sea_orm::{DatabaseConnection, Database};
use tonic::transport::Channel;

struct ServiceState {
    db: DatabaseConnection,
    sandbox: JudgeServiceClient<Channel>,
}

impl ServiceState {
    async fn new() -> Arc<ServiceState> {
        let config=CONFIG.get().unwrap();

        let db=Database::connect(&config.database.uri).await.unwrap();

        let sandbox = JudgeServiceClient::connect("http://127.0.0.1:8080")
            .await
            .unwrap();

        Arc::new(ServiceState {
            db,
            sandbox,
        })
    }
}
