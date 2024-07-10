use sea_orm::{
    ActiveModelTrait, ActiveValue, ConnectOptions, ConnectionTrait, Database, DatabaseBackend,
    DatabaseConnection, EntityTrait, PaginatorTrait, Statement,
};

use super::InitError;
use tracing::{debug_span, instrument, Instrument, Span};

use crate::config::{self};
use crate::{controller::crypto::CryptoController, util::auth::RoleLv};

#[instrument(skip_all, name = "construct_db",parent=span)]
/// initialize the database and connection
///
/// 1. Connect to database.
/// 2. Check and run migration.(skip when not(feature="standalone"))
/// 3. insert user admin@admin if there is no user.
/// 4. return DatabaseConnection
pub async fn init(
    config: &config::Database,
    crypto: &CryptoController,
    span: &Span,
) -> super::Result<DatabaseConnection> {
    let uri = format!("sqlite://{}?mode=rwc&cache=private", config.path.clone());

    let mut opt = ConnectOptions::new(uri);
    opt.sqlx_logging_level(log::LevelFilter::Trace);

    let db = Database::connect(opt).await.map_err(InitError::InitConn)?;

    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        "PRAGMA cache_size = -65536;PRAGMA optimize;", // 64MiB cache
    ))
    .instrument(debug_span!("db_optimize"))
    .await
    .map_err(InitError::OptimizeDB)?;

    #[cfg(feature = "standalone")]
    if config.migrate == Some(true) {
        migrate(&db).await?;
    }

    init_user(&db, crypto).await?;

    Ok(db)
}

#[cfg(feature = "standalone")]
/// Run migration
async fn migrate(db: &DatabaseConnection) -> super::Result<()> {
    sea_orm_migration::cli::run_migrate(
        ::migration::Migrator,
        db,
        Some(sea_orm_cli::cli::MigrateSubcommands::Up { num: None }),
        false,
    )
    .await
    .map_err(InitError::AutoMigrate)?;
    Ok(())
}

#[instrument(skip_all, name = "construct_admin")]
/// check if any user exist or inser user admin@admin
async fn init_user(db: &DatabaseConnection, crypto: &CryptoController) -> super::Result<()> {
    if crate::entity::user::Entity::find().count(db).await.unwrap() != 0 {
        return Ok(());
    }

    tracing::info!("Setting up admin@admin");
    let perm = RoleLv::Root;

    crate::entity::user::ActiveModel {
        permission: ActiveValue::Set(perm as i32),
        username: ActiveValue::Set("admin".to_owned()),
        password: ActiveValue::Set(crypto.hash("admin")),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(InitError::UserCreation)?;

    Ok(())
}
