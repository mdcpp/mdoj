use entity::user;
use sea_orm::{ActiveModelTrait, ActiveValue};

use crate::init::db::DB;

pub struct UserController {}

impl UserController {
    // pub async fn add() {
    //     let db = DB.get().unwrap();
    //     let user = user::ActiveModel {
    //         permission: ActiveValue::Set(0),
    //         username: ActiveValue::Set(()),
    //         password: todo!(),
    //         ..Default::default()
    //     };

    //     user.insert(db).await.unwrap();
    // }
}
