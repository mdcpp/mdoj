use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use crate::problem;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Deserialize, Serialize)]
#[sea_orm(table_name = "submits")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    #[serde(skip_deserializing)]
    pub id: i32,
    #[serde(skip_deserializing)]
    pub user_id: i32,
    #[serde(skip_deserializing)]
    pub problem_id: i32,
    #[sea_orm(ColumnType = "Timestamp")]
    pub begin: String,
    #[sea_orm(ColumnType = "Timestamp")]
    pub end: String,
    pub memory: i64,
    pub pass_case: i32,
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
