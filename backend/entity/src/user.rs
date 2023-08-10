use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{contest, problem, token};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Deserialize, Serialize)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    #[serde(skip_deserializing)]
    pub id: i32,
    #[serde(skip_deserializing)]
    pub submit_id: i32,
    pub permission: i64,
    pub username: String,
    pub hashed_pwd: Vec<u8>,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    // Contest,
    Problem,
    Token,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            // Self::Contest => Entity::has_one(contest::Entity).into(),
            Self::Problem => Entity::has_many(problem::Entity).into(),
            Self::Token => Entity::has_many(token::Entity).into(),
        }
    }
}

// impl Related<contest::Entity> for Entity {
//     fn to() -> RelationDef {
//         Relation::Contest.def()
//     }
// }

impl Related<problem::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Problem.def()
    }
}

impl Related<token::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Token.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
