use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{problem, user};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Deserialize, Serialize)]
#[sea_orm(table_name = "contests")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i32,
    #[sea_orm(indexed)]
    pub hoster: i32,
    #[sea_orm(column_type = "Timestamp")]
    pub begin: DateTime,
    #[sea_orm(column_type = "Timestamp")]
    pub end: DateTime,
    pub title: String,
    pub content: String,
    pub tags: String,
    #[sea_orm(nullable)]
    pub password: Option<String>,
    #[sea_orm(column_type = "Timestamp", on_insert = "current_timestamp")]
    pub create_at: DateTime,
    #[sea_orm(column_type = "Timestamp", on_update = "current_timestamp")]
    pub update_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    // User,
    Users,
    Problems,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            // Self::User => Entity::belongs_to(user::Entity)
            //     .from(Column::UserId)
            //     .to(user::Column::Id)
            //     .into(),
            Self::Users => Entity::has_many(user::Entity).into(),
            Self::Problems => Entity::has_many(problem::Entity).into(),
        }
    }
}

impl Related<problem::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Problems.def()
    }
}

impl Related<user::Entity> for Entity {
    fn to() -> RelationDef {
        super::user_contest::Relation::User.def()
    }
    fn via() -> Option<RelationDef> {
        Some(super::user_contest::Relation::Contest.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}
