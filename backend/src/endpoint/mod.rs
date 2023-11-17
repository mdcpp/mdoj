use tonic::async_trait;

pub mod problem;
pub mod testcase;
pub mod util;

pub mod tools {
    pub use super::*;
    pub use util::error::{Error, ResultExt};
    // pub use util::error::handle_dberr;
    pub use super::util::auth::Auth;
    pub use super::util::ControllerTrait;
    pub use crate::init::db::DB;
}
pub mod endpoints {
    pub use super::*;
    pub use crate::{impl_create_request, impl_endpoint, impl_intel, impl_update_request};
    pub use sea_orm::{
        ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, IntoActiveModel, ModelTrait,
        PaginatorTrait, QueryFilter, QuerySelect,
    };
}
