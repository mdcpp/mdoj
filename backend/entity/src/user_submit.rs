use sea_orm::prelude::*;

use crate::{submit, user};

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    User,
    Submit,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Relation::User => submit::Entity::has_many(user::Entity).into(),
            Relation::Submit => user::Entity::has_many(submit::Entity).into(),
        }
    }
}

impl Related<submit::Entity> for user::Entity {
    fn to() -> RelationDef {
        Relation::Submit.def()
    }

    fn via() -> Option<RelationDef> {
        Some(Relation::User.def().rev())
    }
}

impl Related<user::Entity> for submit::Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }

    fn via() -> Option<RelationDef> {
        Some(Relation::Submit.def().rev())
    }
}
