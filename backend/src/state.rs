use sea_orm::DatabaseConnection;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct AppState {
    pub db_conn: Arc<DatabaseConnection>,
}

pub async fn generate_state() -> AppState {
    let uri = std::env::var("POSTGRES")
        .unwrap_or("postgres://postgres:admin@localhost/postgres".to_owned());
    let db_conn: DatabaseConnection = sea_orm::Database::connect(uri).await.unwrap();
    AppState {
        db_conn: Arc::new(db_conn),
    }
}
