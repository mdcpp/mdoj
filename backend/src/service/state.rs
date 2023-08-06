// use std::sync::Arc;

// use crate::{init::config::CONFIG, proto::prelude::judge_service_client::JudgeServiceClient};
// use sea_orm::{Database, DatabaseConnection};
// use tonic::transport::Channel;

// struct ServiceState {
//     db: DatabaseConnection,
//     sandbox: JudgeServiceClient<Channel>,
// }

// impl ServiceState {
//     async fn new() -> Arc<ServiceState> {
//         let config = CONFIG.get().unwrap();

//         let db = Database::connect(&config.database.uri).await.unwrap();

//         let sandbox = JudgeServiceClient::connect("http://127.0.0.1:8080")
//             .await
//             .unwrap();

//         Arc::new(ServiceState { db, sandbox })
//     }
// }
