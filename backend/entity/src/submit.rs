use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use crate::problem;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Deserialize, Serialize)]
#[sea_orm(table_name = "submits")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i32,
    #[sea_orm(nullable)]
    pub user_id: Option<i32>,
    pub problem_id: i32,
    #[sea_orm(column_type = "Timestamp", on_insert = "current_timestamp", indexed)]
    pub upload_at: DateTime,
    #[sea_orm(nullable, indexed)]
    pub time: Option<u64>,
    #[sea_orm(nullable)]
    pub accuracy: Option<u64>,
    #[sea_orm(default_value = "false")]
    pub committed: bool,
    pub lang: String,
    pub code: Vec<u8>,
    #[sea_orm(nullable)]
    pub memory: Option<u64>,
    #[sea_orm(default_value = 0, indexed)]
    pub pass_case: i32,
    #[sea_orm(default_value = false)]
    pub accept: bool,
    #[sea_orm(default_value = 0, indexed)]
    pub score: u32,
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
