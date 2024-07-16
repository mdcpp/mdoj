use std::fmt::Display;

use grpc::backend::Role;

use super::error::Error;

/// Role Level
///
/// The greater the value, the greater the permission
///
/// - Guest: Read only
/// - User: Join contest, submit code, chat
/// - Super: Create contest, Create problem
/// - Admin: Manage user(cannot create Root), Manage contest, Manage problem
/// - Root: Manage everything
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

/// Authication
///
/// The difference between [`Auth`] and [`RoleLv`] is that
/// [`Auth`] contain user id, and [`RoleLv`] is just permmision
#[derive(Debug)]
pub enum Auth {
    Guest,
    User((i32, RoleLv)),
}

impl Display for Auth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Auth::Guest => write!(f, "Guest"),
            Auth::User((uid, role)) => write!(f, "{}({})", role, uid),
        }
    }
}

impl Auth {
    /// check if the user is guest(not signed in)
    pub fn is_guest(&self) -> bool {
        matches!(self, Auth::Guest)
    }
    /// get the user's permission level
    pub fn user_perm(&self) -> RoleLv {
        match self {
            Auth::User((_, x)) => *x,
            _ => RoleLv::Guest,
        }
    }
    /// get the user's id if signed in
    pub fn user_id(&self) -> Option<i32> {
        match self {
            Auth::User((x, _)) => Some(*x),
            _ => None,
        }
    }
    /// destruct the Auth into user id and permission level
    pub fn into_inner(&self) -> Option<(i32, RoleLv)> {
        match self {
            Auth::User(x) => Some(*x),
            _ => None,
        }
    }
    /// short hand for `self.into_inner().ok_or(err)`
    pub fn auth_or_error(&self, err: Error) -> Result<(i32, RoleLv), Error> {
        self.into_inner().ok_or(err)
    }
    /// short hand for `self.into_inner().ok_or(Error::PermissionDeny)`
    pub fn auth_or_guest(&self) -> Result<(i32, RoleLv), Error> {
        self.into_inner().ok_or(Error::PermissionDeny(
            "Only signed in user is allow in this endpoint",
        ))
    }
}
