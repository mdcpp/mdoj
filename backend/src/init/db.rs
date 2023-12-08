use std::path::PathBuf;
use std::sync::Arc;

use ring::digest;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ConnectionTrait, Database, DatabaseConnection, EntityTrait,
    PaginatorTrait, Schema,
};
use tokio::fs;
use tokio::sync::OnceCell;

use super::config::{self, GlobalConfig};
use crate::controller::token::UserPermBytes;

pub static DB: OnceCell<DatabaseConnection> = OnceCell::const_new();

pub async fn init(config: &config::Database) {
    // sqlite://database/backend.sqlite?mode=rwc
    let uri = format!("sqlite://{}", config.path.clone());

    let db = Database::connect(&uri)
        .await
        .expect("fail connecting to database");
    init_user(config, &db).await;
    DB.set(db).ok();
}
fn hash(config: &config::Database, src: &str) -> Vec<u8> {
    digest::digest(
        &digest::SHA256,
        &[src.as_bytes(), config.salt.as_bytes()].concat(),
    )
    .as_ref()
    .to_vec()
}

pub async fn init_user(config: &config::Database, db: &DatabaseConnection) {
    if entity::user::Entity::find().count(db).await.unwrap() != 0 {
        return;
    }

    log::info!("Setting up admin@admin");
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
