//! `SeaORM` Entity, @generated by sea-orm-codegen 1.0.0

use super::*;
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "tag")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub name: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::tag_problem::Entity")]
    TagProblem,
}

impl Related<tag_problem::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TagProblem.def()
    }
}

impl Related<problem::Entity> for Entity {
    fn to() -> RelationDef {
        tag_problem::Relation::Problem.def()
    }
    fn via() -> Option<RelationDef> {
        Some(tag_problem::Relation::Tag.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}
