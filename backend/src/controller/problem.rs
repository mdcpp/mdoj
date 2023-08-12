use std::collections::HashMap;

use entity::problem;
use sea_orm::{ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter};
use tokio::sync::watch;

use crate::init::db::DB;

use super::{util::router, Error};

pub struct ProblemBase {
    pub title: String,
    pub owner: i32,
}

// pub struct ProblemUpdate{
//     title:Option<String>,
//     description:Option<String>,
// }

pub struct ProblemController {
    judgers: router::JudgeRouter,
    running_submits: HashMap<i32, watch::Receiver<i32>>,
}

impl ProblemController {
    pub async fn add(&self, base: ProblemBase) -> Result<problem::Model, Error> {
        let db = DB.get().unwrap();

        let problem = problem::ActiveModel {
            title: ActiveValue::Set(base.title),
            user_id: ActiveValue::Set(base.owner),
            ..Default::default()
        }
        .insert(db)
        .await?;

        Ok(problem)
    }
    pub async fn remove(&self, problem_id: i32) -> Result<Option<()>, Error> {
        let db = DB.get().unwrap();

        let problem = problem::Entity::delete_many()
            .filter(problem::Column::Id.eq(problem_id))
            .exec(db)
            .await?;

        Ok(match problem.rows_affected == 0 {
            true => Some(()),
            false => None,
        })
    }
    // pub async fn submit(&self){

    // }
    // pub async fn update()->
}
