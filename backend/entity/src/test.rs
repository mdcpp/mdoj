use sea_orm::{entity::prelude::*, FromQueryResult};
use serde::{Deserialize, Serialize};

use crate::problem;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Deserialize, Serialize)]
#[sea_orm(table_name = "test")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i32,
    #[sea_orm(nullable, indexed)]
    pub user_id: i32,
    #[sea_orm(nullable, indexed)]
    pub problem_id: Option<i32>,
    pub input: Vec<u8>,
    pub output: Vec<u8>,
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

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Tests(Vec<Vec<u8>>);

impl Tests {
    pub fn new(i: Vec<Vec<u8>>) -> Self {
        Self(i)
    }
    pub fn from_raw(raw: Vec<u8>) -> Self {
        let tests: Self = bincode::deserialize(&raw).unwrap();
        tests
    }
    pub fn into_raw(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }
}

#[derive(DerivePartialModel, FromQueryResult)]
#[sea_orm(entity = "Entity")]
pub struct PartialTestcase {
    #[sea_orm(from_col = "id")]
    pub id: i32,
    pub user_id: i32,
    pub problem_id: i32,
}
