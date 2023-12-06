use std::path::PathBuf;

use ring::digest;
use sea_orm::{ActiveModelTrait, ActiveValue, Database, DatabaseConnection};
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
            println!("Database connection failed, creating database");

            fs::File::create(PathBuf::from(config.database.path.clone()))
                .await
                .unwrap();

            let db: DatabaseConnection = Database::connect(&uri).await.unwrap();

            first_migration(config,&db).await;
            DB.set(db).unwrap();

            println!("Database created");
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

pub async fn first_migration(config: &GlobalConfig,db:&DatabaseConnection) {
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
    .save(db)
    .await
    .unwrap();
}
