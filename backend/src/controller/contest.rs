use entity::{contest, user, user_contest};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, ModelTrait, QueryFilter};

use crate::init::db::DB;

use super::Error;

// todo!() cache
pub struct ContestController {}

impl ContestController {
    pub async fn has_participate(&self, contest_id: i32, user_id: i32) -> Result<bool, Error> {
        let db = DB.get().unwrap();

        let query = contest::Entity::find_by_id(contest_id)
            .find_with_related(user::Entity)
            .filter(user::Column::Id.eq(user_id))
            .all(db)
            .await?;

        match query.len() {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(Error::Corrupted),
        }
    }
    pub async fn list_participate(&self, contest_id: i32) -> Result<Vec<user::Model>, Error> {
        let db = DB.get().unwrap();

        let contest = contest::Entity::find_by_id(contest_id).one(db).await?;

        let users: Vec<user::Model> = contest
            .ok_or(Error::NotFound("User"))?
            .find_related(user::Entity)
            .all(db)
            .await?;

        Ok(users)
    }
    pub async fn add_participate(&self, contest_id: i32, user_id: i32) -> Result<(), Error> {
        let db = DB.get().unwrap();

        let pivot: user_contest::ActiveModel = user_contest::Model {
            user_id,
            contest_id,
        }
        .into();

        pivot.insert(db).await?;

        Ok(())
    }
    pub fn update(&self) {
        todo!()
    }
}
