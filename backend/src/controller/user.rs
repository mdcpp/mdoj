use entity::user;
use sea_orm::{ActiveModelTrait, ActiveValue};

use crate::init::db::DB;

pub struct UserController {}

impl UserController {
    pub async fn add(username: String, pwd: Vec<u8>) {
        let hashed_pwd=sha256::digest(pwd);
        let db = DB.get().unwrap();
        let user = user::ActiveModel {
            permission: ActiveValue::Set(i64::MIN),
            username: ActiveValue::Set(username),
            hashed_pwd: ActiveValue::Set(hashed_pwd.as_bytes().to_vec()),
            ..Default::default()
        };

        user.insert(db).await.unwrap();
    }
}
