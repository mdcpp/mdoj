use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use crate::problem;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Deserialize, Serialize)]
#[sea_orm(table_name = "submits")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i32,
    pub user_id: i32,
    pub problem_id: i32,
    #[sea_orm(column_type = "Timestamp")]
    pub upload: DateTime,
    #[sea_orm(nullable)]
    pub time: Option<u64>,
    #[sea_orm(default_value = "false")]
    pub committed: bool,
    pub lang: String,
    pub code: Vec<u8>,
    #[sea_orm(nullable)]
    pub memory: Option<i64>,
    #[sea_orm(default_value = 0)]
    pub pass_case: i32,
    #[sea_orm(default_value = 0)]
    pub report: i32,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    Problem,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::Problem => Entity::belongs_to(problem::Entity)
                .from(Column::ProblemId)
                .to(problem::Column::Id)
                .into(),
        }
    }
}

impl Related<problem::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Problem.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
