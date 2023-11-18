use super::endpoints::*;
use super::tools::*;

use crate::grpc::backend::user_set_server::*;
use crate::grpc::backend::*;

use entity::{user::*, *};

impl From<i32> for UserId {
    fn from(value: i32) -> Self {
        Self { id: value }
    }
}

impl From<UserId> for i32 {
    fn from(value: UserId) -> Self {
        value.id
    }
}
