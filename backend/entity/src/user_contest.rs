//! The `user_contest` entity.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// CakeFilling model
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Deserialize, Serialize)]
#[sea_orm(table_name = "user_contest")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub user_id: i32,
    #[sea_orm(primary_key, auto_increment = false)]
    pub contest_id: i32,
}

/// CakeFilling relation
#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    /// Cake relation
    User,
    /// Filling relation
    Contest,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::User => Entity::belongs_to(super::user::Entity)
                .from(Column::UserId)
                .to(super::user::Column::Id)
                .into(),
            Self::Contest => Entity::belongs_to(super::contest::Entity)
                .from(Column::ContestId)
                .to(super::contest::Column::Id)
                .into(),
        }
    }
}

impl ActiveModelBehavior for ActiveModel {}
