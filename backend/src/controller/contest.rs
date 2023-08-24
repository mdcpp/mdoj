use entity::{contest, user};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use crate::init::db::DB;

use super::Error;

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
    pub fn list_participate(&self, contest_id: i32) {
        todo!()
    }
    pub fn add_participate(&self, contest_id: i32, user: user::Model) -> Result<(), Error> {
        todo!()
    }
    // Shouldn't it be updated directly(orm)?
    // Due to Tasks::from_raw, problem is heavily wrapped for example
    pub fn update(){
        todo!()
    }
}
