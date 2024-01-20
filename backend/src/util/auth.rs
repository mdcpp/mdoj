use std::fmt::Display;

use crate::{entity::user, grpc::backend::Role};
use sea_orm::{EntityTrait, QuerySelect};

use super::error::Error;

#[derive(Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Debug)]
#[repr(i32)]
pub enum RoleLv {
    Guest = 0,
    User = 1,
    Super = 2,
    Admin = 3,
    Root = 4,
}

impl Display for RoleLv {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RoleLv::Guest => write!(f, "\"Guest\""),
            RoleLv::User => write!(f, "\"User\""),
            RoleLv::Super => write!(f, "\"Super User\""),
            RoleLv::Admin => write!(f, "\"Admin\""),
            RoleLv::Root => write!(f, "\"Root\""),
        }
    }
}

impl From<Role> for RoleLv {
    fn from(value: Role) -> Self {
        match value {
            Role::User => RoleLv::User,
            Role::Super => RoleLv::Super,
            Role::Admin => RoleLv::Admin,
            Role::Root => RoleLv::Root,
        }
    }
}

impl TryFrom<i32> for RoleLv {
    type Error = super::error::Error;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Guest),
            1 => Ok(Self::User),
            2 => Ok(Self::Super),
            3 => Ok(Self::Admin),
            4 => Ok(Self::Root),
            _ => Err(Error::Unreachable("Invaild RoleLv")),
        }
    }
}

impl RoleLv {
    pub fn user(&self) -> bool {
        *self as i32 >= 1
    }
    pub fn super_user(&self) -> bool {
        *self as i32 >= 2
    }
    pub fn admin(&self) -> bool {
        *self as i32 >= 3
    }
    pub fn root(&self) -> bool {
        *self as i32 >= 4
    }
}

pub enum Auth {
    Guest,
    User((i32, RoleLv)),
}

impl Auth {
    pub fn is_guest(&self) -> bool {
        matches!(self, Auth::Guest)
    }
    pub fn user_perm(&self) -> RoleLv {
        match self {
            Auth::User((_, x)) => *x,
            _ => RoleLv::Guest,
        }
    }
    pub fn user_id(&self) -> Option<i32> {
        match self {
            Auth::User((x, _)) => Some(*x),
            _ => None,
        }
    }
    pub fn ok_or(&self, err: Error) -> Result<(i32, RoleLv), Error> {
        match self {
            Auth::User(x) => Ok(*x),
            _ => Err(err),
        }
    }
    pub fn ok_or_default(&self) -> Result<(i32, RoleLv), Error> {
        self.ok_or(Error::PermissionDeny(
            "Only signed in user is allow in this endpoint",
        ))
    }
    pub async fn get_user(&self, db: &sea_orm::DatabaseConnection) -> Result<user::Model, Error> {
        let user_id = self.user_id().ok_or(Error::Unauthenticated)?;
        user::Entity::find_by_id(user_id)
            .columns([user::Column::Id])
            .one(db)
            .await?
            .ok_or(Error::NotInDB("user"))
    }
}

// X-Forwarded-For
