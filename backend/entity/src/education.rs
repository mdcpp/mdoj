use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use crate::user;

use super::problem;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Deserialize, Serialize)]
#[sea_orm(table_name = "education")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i32,
    pub problem_id: i32,
    pub user_id: i32,
    pub content: String,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    Problem,
    User,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Relation::Problem => Entity::belongs_to(problem::Entity)
                .from(Column::ProblemId)
                .to(problem::Column::Id)
                .into(),
            Relation::User => Entity::belongs_to(user::Entity)
                .from(Column::UserId)
                .to(user::Column::Id)
                .into(),
        }
    }
}

impl Related<problem::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Problem.def()
    }
}

impl Related<user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
