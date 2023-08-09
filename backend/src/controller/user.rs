use entity::user;
use sea_orm::{ActiveModelTrait, ActiveValue};

use crate::init::db::DB;

pub struct UserController {}

impl UserController {
    pub async fn add(username: String, hashed_pwd: Vec<u8>) {
        let db = DB.get().unwrap();
        let user = user::ActiveModel {
            permission: ActiveValue::Set(i64::MIN),
            username: ActiveValue::Set(username),
            hashed_pwd: ActiveValue::Set(hashed_pwd),
            ..Default::default()
        };

        user.insert(db).await.unwrap();
    }
}
