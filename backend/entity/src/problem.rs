use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{contest, education, submit, user};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Deserialize, Serialize)]
#[sea_orm(table_name = "problem")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i32,
    pub user_id: i32,
    pub contest_id: i32,
    pub success: i32,
    pub submits: i32,
    pub memory: i64,
    pub time: u64,
    pub tests: Vec<u8>,
    pub tags: String,
    pub title: String,
    pub description: String,
    pub visible: bool,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    User,
    Contest,
    Submit,
    Education,
    // TestCase,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::User => Entity::belongs_to(user::Entity)
                .from(Column::UserId)
                .to(user::Column::Id)
                .into(),
            Self::Contest => Entity::belongs_to(contest::Entity)
                .from(Column::UserId)
                .to(contest::Column::Id)
                .into(),
            Self::Submit => Entity::has_many(submit::Entity).into(),
            Self::Education => Entity::has_many(education::Entity).into(),
            // Self::TestCase => Entity::has_many(testcase::Entity).into(),
        }
    }
}

impl Related<user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<contest::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Contest.def()
    }
}

impl Related<submit::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Submit.def()
    }
}
impl Related<education::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Submit.def()
    }
}

// impl Related<testcase::Entity> for Entity {
//     fn to() -> RelationDef {
//         Relation::TestCase.def()
//     }
// }

impl ActiveModelBehavior for ActiveModel {}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Tests(pub (Vec<(Vec<u8>, Vec<u8>)>));

impl Tests {
    pub fn from_raw(raw: Vec<u8>) -> Self {
        let tests: Self = bincode::deserialize(&raw).unwrap();
        tests
    }
    pub fn into_raw(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }
}
