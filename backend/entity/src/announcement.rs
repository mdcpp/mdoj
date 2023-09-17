use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Deserialize, Serialize)]
#[sea_orm(table_name = "announcement")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i32,
    pub title: String,
    pub content: String,
    #[sea_orm(column_type = "Timestamp", on_insert = "current_timestamp")]
    pub create_at: DateTime,
    #[sea_orm(column_type = "Timestamp", on_update = "current_timestamp")]
    pub update_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        panic!("No relation for this entity")
    }
}

impl ActiveModelBehavior for ActiveModel {}
