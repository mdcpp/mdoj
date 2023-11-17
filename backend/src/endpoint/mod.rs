use tonic::async_trait;

pub mod problem;
pub mod testcase;
pub mod util;

pub mod tools {
    pub use super::util::auth::Auth;
    pub use super::*;
    pub use crate::init::db::DB;
    pub use util::error::Error;
}
pub mod endpoints {
    pub use super::*;
    pub use sea_orm::{
        ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, IntoActiveModel, ModelTrait,
        PaginatorTrait, QueryFilter, QuerySelect,
    };
}
