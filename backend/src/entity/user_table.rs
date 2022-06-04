//! SeaORM Entity. Generated by sea-orm-codegen 0.8.0

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "user_table")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub create_time: Option<Date>,
    pub update_time: Option<Date>,
    pub name_user: String,
    pub privilege: i32,
    pub hashed_password: Vec<u8>,
    pub description: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::token_table::Entity")]
    TokenTable,
    #[sea_orm(has_many = "super::question_user::Entity")]
    QuestionUser,
}

impl Related<super::token_table::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TokenTable.def()
    }
}

impl Related<super::question_user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::QuestionUser.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
