use sea_orm::DatabaseConnection;
use std::sync::{Arc, Mutex};

// use crate::controller::Cache;
// use crate::controller::crypto::Cache;

#[derive(Debug, Clone)]
pub struct AppState {
    pub db_conn: Arc<DatabaseConnection>,
    // pub token_cache: Arc<Cache>,
}

pub async fn generate_state() -> AppState {
    let uri = std::env::var("POSTGRES")
        .unwrap_or("postgres://postgres:postgres@192.168.1.199/postgres".to_owned());
    let db_conn: DatabaseConnection = sea_orm::Database::connect(uri).await.unwrap();
    AppState {
        db_conn: Arc::new(db_conn),
        // token_cache: Arc::new(Cache::new(100)),
    }
}
