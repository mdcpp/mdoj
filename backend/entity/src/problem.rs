use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{contest, submit, testcase, user};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Deserialize, Serialize)]
#[sea_orm(table_name = "problem")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    #[serde(skip_deserializing)]
    pub id: i32,
    #[serde(skip_deserializing)]
    pub user_id: i32,
    #[serde(skip_deserializing)]
    pub contest_id: i32,
    pub success: i32,
    pub submits: i32,
    pub title: String,
    pub description: String,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    User,
    Contest,
    Submit,
    TestCase,
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
            Self::TestCase => Entity::has_many(testcase::Entity).into(),
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

impl Related<testcase::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TestCase.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
