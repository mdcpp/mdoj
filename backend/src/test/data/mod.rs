use sea_orm::{Database, DatabaseConnection};
use sea_orm_cli::cli::MigrateSubcommands;
use tonic::async_trait;

pub mod paginator;
pub mod problem;

#[async_trait]
pub trait Data {
    async fn insert(db: &DatabaseConnection);
    async fn connect() -> DatabaseConnection {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        sea_orm_migration::cli::run_migrate(
            migration::Migrator,
            &db,
            Some(MigrateSubcommands::Init),
            false,
        )
        .await
        .unwrap();

        Self::insert(&db).await;

        db
    }
}
