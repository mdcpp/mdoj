use sea_orm::{Database, DatabaseConnection};
use tokio::sync::OnceCell;

use super::config::CONFIG;

pub static DB: OnceCell<DatabaseConnection> = OnceCell::const_new();

pub async fn init() {
    let config = CONFIG.get().unwrap();

    let db: DatabaseConnection = Database::connect(config.database.uri.clone())
        .await
        .unwrap();

    DB.set(db).unwrap();

    if std::env::var_os("INIT").is_some() {
        log::info!("Empty database detected, init database");

        todo!("add root user");
    }
}
