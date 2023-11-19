use sea_orm::{ActiveModelTrait, ActiveValue, Database, DatabaseConnection};
use tokio::sync::OnceCell;

use super::config::CONFIG;
use crate::controller::token::UserPermBytes;
use crate::endpoint::util::hash;

pub static DB: OnceCell<DatabaseConnection> = OnceCell::const_new();

pub async fn init() {
    let config = CONFIG.get().unwrap();

    let db: DatabaseConnection = Database::connect(config.database.uri.clone())
        .await
        .unwrap();

    DB.set(db).unwrap();
}

pub async fn first_migration() {
    let db = DB.get().unwrap();
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
        password: ActiveValue::Set(hash::hash("admin")),
        ..Default::default()
    }
    .save(db)
    .await
    .unwrap();
}
