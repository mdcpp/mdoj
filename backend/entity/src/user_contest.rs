use sea_orm::prelude::*;

use crate::{contest, user};

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    User,
    Contest,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Relation::User => contest::Entity::has_many(user::Entity).into(),
            Relation::Contest => user::Entity::has_many(contest::Entity).into(),
        }
    }
}

impl Related<contest::Entity> for user::Entity {
    fn to() -> RelationDef {
        Relation::Contest.def()
    }

    fn via() -> Option<RelationDef> {
        Some(Relation::User.def().rev())
    }
}

impl Related<user::Entity> for contest::Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }

    fn via() -> Option<RelationDef> {
        Some(Relation::Contest.def().rev())
    }
}
