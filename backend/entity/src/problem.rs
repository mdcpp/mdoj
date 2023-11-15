use sea_orm::{entity::prelude::*, DerivePartialModel, FromQueryResult};
use serde::{Deserialize, Serialize};

use crate::{contest, education, submit, test, user};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Deserialize, Serialize)]
#[sea_orm(table_name = "problem")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i32,
    pub user_id: i32,
    pub contest_id: i32,
    #[sea_orm(default_value = 0)]
    pub success: i32,
    #[sea_orm(default_value = 0)]
    pub submit_count: u32,
    #[sea_orm(default_value = 0.0)]
    pub ac_rate: f32,
    pub memory: u64,
    pub time: u64,
    pub difficulty: u32,
    pub public: bool,
    pub tags: String,
    pub title: String,
    pub content: String,
    #[sea_orm(column_type = "Timestamp", on_insert = "current_timestamp")]
    pub create_at: DateTime,
    #[sea_orm(column_type = "Timestamp", on_update = "current_timestamp")]
    pub update_at: DateTime,
    pub match_rule: i32,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    User,
    Contest,
    Submit,
    Education,
    TestCase,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::User => Entity::has_one(user::Entity)
                .from(Column::UserId)
                .to(user::Column::Id)
                .into(),
            Self::Contest => Entity::belongs_to(contest::Entity)
                .from(Column::ContestId)
                .to(contest::Column::Id)
                .into(),
            Self::Submit => Entity::has_many(submit::Entity).into(),
            Self::Education => Entity::has_one(education::Entity).into(),
            Self::TestCase => Entity::has_many(test::Entity).into(),
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

impl Related<test::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TestCase.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

type Problem = Entity;
#[derive(DerivePartialModel, FromQueryResult)]
#[sea_orm(entity = "Problem")]
pub struct PartialTestcase {
    #[sea_orm(from_col = "id")]
    pub id: i32,
    pub title: String,
    pub submit_count: u32,
    pub ac_rate: f32,
}
