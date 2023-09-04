use entity::{token, user};
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DbErr, EntityTrait, QueryFilter, Related,
    TransactionTrait,
};

use crate::init::db::DB;

use super::Error;

pub struct UserController {}

impl UserController {
    pub async fn add(&self, username: String, pwd: Vec<u8>) -> Result<(), Error> {
        let hashed_pwd = sha256::digest(pwd);
        let db = DB.get().unwrap();
        let user = user::ActiveModel {
            permission: ActiveValue::Set(i64::MIN),
            username: ActiveValue::Set(username),
            hashed_pwd: ActiveValue::Set(hashed_pwd.as_bytes().to_vec()),
            ..Default::default()
        };

        user.insert(db).await?;

        Ok(())
    }
    pub async fn delete(&self, user_id: i32) -> Result<Option<user::Model>, Error> {
        let db = DB.get().unwrap();

        Ok(db
            .transaction::<_, Option<user::Model>, DbErr>(|txn| {
                Box::pin(async move {
                    if let Some(user) = user::Entity::find_by_id(user_id).one(txn).await.unwrap() {
                        token::Entity::delete_many()
                            .filter(token::Column::UserId.eq(user.id))
                            .exec(txn)
                            .await
                            .unwrap();
                        user::Entity::delete_by_id(user_id).exec(txn).await.unwrap();

                        Ok(Some(user))
                    } else {
                        Ok(None)
                    }
                })
            })
            .await?)
    }
}
