use std::path::PathBuf;

use sea_orm::{
    ActiveModelTrait, ActiveValue, ConnectionTrait, Database, DatabaseBackend, DatabaseConnection,
    EntityTrait, PaginatorTrait, Statement,
};

use sea_orm_cli::MigrateSubcommands;
use sea_orm_migration::cli::*;
use tokio::{fs, sync::OnceCell};
use tracing::{debug_span, instrument, Instrument, Span};

use super::config::{self};
use crate::{controller::crypto::CryptoController, util::auth::RoleLv};

pub static DB: OnceCell<DatabaseConnection> = OnceCell::const_new();

#[instrument(skip_all, name = "construct_db",parent=span)]
pub async fn init(config: &config::Database, crypto: &CryptoController, span: &Span) {
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

    #[cfg(feature = "standalone")]
    if config.migrate == Some(true) {
        migrate(&db).await;
    }

    init_user(&db, crypto).await;

    DB.set(db).ok();
}

#[cfg(feature = "standalone")]
async fn migrate(db: &DatabaseConnection) {
    run_migrate(
        ::migration::Migrator,
        db,
        Some(MigrateSubcommands::Up { num: None }),
        false,
    )
    .await
    .expect("Unable to setup database migration");
}

#[instrument(skip_all, name = "construct_admin")]
async fn init_user(db: &DatabaseConnection, crypto: &CryptoController) {
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
