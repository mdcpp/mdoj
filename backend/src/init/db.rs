use sea_orm::{
    ActiveModelTrait, ActiveValue, ConnectionTrait, Database, DatabaseBackend, DatabaseConnection,
    EntityTrait, PaginatorTrait, Statement,
};

use tokio::sync::OnceCell;
use tracing::{debug_span, instrument, Instrument};

use super::config::{self};
use crate::controller::{crypto::CryptoController, token::UserPermBytes};

pub static DB: OnceCell<DatabaseConnection> = OnceCell::const_new();

#[instrument(skip_all, name = "construct_db")]
pub async fn init(config: &config::Database, crypto: &CryptoController) {
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
    if entity::user::Entity::find().count(db).await.unwrap() != 0 {
        return;
    }

    tracing::info!("Setting up admin@admin");
    let mut perm = UserPermBytes::default();

    perm.grant_link(true);
    perm.grant_root(true);
    perm.grant_publish(true);
    perm.grant_manage_announcement(true);
    perm.grant_manage_education(true);
    perm.grant_manage_problem(true);
    perm.grant_manage_submit(true);
    perm.grant_manage_contest(true);

    entity::user::ActiveModel {
        permission: ActiveValue::Set(perm.0),
        username: ActiveValue::Set("admin".to_owned()),
        password: ActiveValue::Set(crypto.hash("admin").into()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
}
