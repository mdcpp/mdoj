use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{contest, education, problem, token};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Deserialize, Serialize)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i32,
    pub submit_id: i32,
    pub permission: i64,
    pub username: String,
    pub hashed_pwd: Vec<u8>,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    // Contest,
    Contests,
    Problems,
    Tokens,
    Education,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            // Self::Contest => Entity::has_many(contest::Entity).into(),
            Self::Contests => Entity::has_many(contest::Entity).into(),
            Self::Problems => Entity::has_many(problem::Entity).into(),
            Self::Tokens => Entity::has_many(token::Entity).into(),
            Self::Education => Entity::has_many(education::Entity).into(),
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
        Relation::Problems.def()
    }
}

impl Related<token::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Tokens.def()
    }
}
impl Related<education::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Education.def()
    }
}

impl Related<contest::Entity> for Entity {
    fn to() -> RelationDef {
        super::user_contest::Relation::Contest.def()
    }
    fn via() -> Option<RelationDef> {
        Some(super::user_contest::Relation::User.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}
