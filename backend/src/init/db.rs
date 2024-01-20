use sea_orm::{
    ActiveModelTrait, ActiveValue, ConnectionTrait, Database, DatabaseBackend, DatabaseConnection,
    EntityTrait, PaginatorTrait, Statement,
};

use tokio::sync::OnceCell;
use tracing::{debug_span, instrument, Instrument, Span};

use super::config::{self};
use crate::{controller::crypto::CryptoController, util::auth::RoleLv};

pub static DB: OnceCell<DatabaseConnection> = OnceCell::const_new();

#[instrument(skip_all, name = "construct_db",parent=span)]
pub async fn init(config: &config::Database, crypto: &CryptoController, span: &Span) {
    // sqlite://database/backend.sqlite?mode=rwc
    let uri = format!("sqlite://{}?mode=rwc&cache=private", config.path.clone());

    let db = Database::connect(&uri)
        .await
        .expect("fail connecting to database");

    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        "PRAGMA cache_size = -65536;PRAGMA optimize;", // 64MiB cache
    ))
    .instrument(debug_span!("db_optimize"))
    .await
    .unwrap();

    init_user(&db, crypto).await;

    DB.set(db).ok();
}
// fn hash(config: &config::Database, src: &str) -> Vec<u8> {
//     digest::digest(
//         &digest::SHA256,
//         &[src.as_bytes(), config.salt.as_bytes()].concat(),
//     )
//     .as_ref()
//     .to_vec()
// }

#[instrument(skip_all, name = "construct_admin")]
pub async fn init_user(db: &DatabaseConnection, crypto: &CryptoController) {
    if crate::entity::user::Entity::find().count(db).await.unwrap() != 0 {
        return;
    }

    tracing::info!("Setting up admin@admin");
    let perm = RoleLv::Root;

    crate::entity::user::ActiveModel {
        permission: ActiveValue::Set(perm as i32),
        username: ActiveValue::Set("admin".to_owned()),
        password: ActiveValue::Set(crypto.hash("admin").into()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
}
