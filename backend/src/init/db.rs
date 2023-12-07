use std::path::PathBuf;

use ring::digest;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ConnectionTrait, Database, DatabaseConnection, EntityTrait,
    Schema,
};
use tokio::fs;
use tokio::sync::OnceCell;

use super::config::GlobalConfig;
use crate::controller::token::UserPermBytes;

pub static DB: OnceCell<DatabaseConnection> = OnceCell::const_new();

pub async fn init(config: &GlobalConfig) {
    let uri = format!("sqlite://{}", config.database.path.clone());

    match Database::connect(&uri).await {
        Ok(db) => {
            DB.set(db).unwrap();
        }
        Err(_) => {
            log::info!("Database connection failed, creating database");

            fs::File::create(PathBuf::from(config.database.path.clone()))
                .await
                .unwrap();

            let db: DatabaseConnection = Database::connect(&uri).await.unwrap();

            first_migration(config, &db).await;

            DB.set(db).unwrap();

            log::info!("Database created");
        }
    }
}
fn hash(config: &GlobalConfig, src: &str) -> Vec<u8> {
    digest::digest(
        &digest::SHA256,
        &[src.as_bytes(), config.database.salt.as_bytes()].concat(),
    )
    .as_ref()
    .to_vec()
}

async fn create_table<E>(db: &DatabaseConnection, entity: E)
where
    E: EntityTrait,
{
    log::info!("Creating table: {}", entity.table_name());
    let builder = db.get_database_backend();
    let stmt = builder.build(
        Schema::new(builder)
            .create_table_from_entity(entity)
            .if_not_exists(),
    );

    match db.execute(stmt).await {
        Ok(_) => log::info!("Migrated {}", entity.table_name()),
        Err(e) => log::info!("Error: {}", e),
    }
}

pub async fn first_migration(config: &GlobalConfig, db: &DatabaseConnection) {
    log::info!("Start migration");
    // create tables
    create_table(db, entity::user::Entity).await;
    create_table(db, entity::token::Entity).await;
    create_table(db, entity::announcement::Entity).await;
    create_table(db, entity::contest::Entity).await;
    create_table(db, entity::education::Entity).await;
    create_table(db, entity::problem::Entity).await;
    create_table(db, entity::submit::Entity).await;
    create_table(db, entity::test::Entity).await;
    create_table(db, entity::user_contest::Entity).await;

    // generate admin@admin
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
        password: ActiveValue::Set(hash(config, "admin")),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
}
