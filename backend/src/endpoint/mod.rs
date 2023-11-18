pub mod contest;
pub mod problem;
pub mod testcase;
pub mod user;
pub mod util;

pub mod tools {
    pub use super::util::auth::Auth;
    pub use super::util::error::Error;
    pub use crate::init::db::DB;
}
pub mod endpoints {
    pub use super::util::{
        filter::{Filter, ParentalFilter},
        pagination::*,
    };
    pub use crate::{fill_active_model, fill_exist_active_model, server::Server};
    pub use sea_orm::{
        ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, IntoActiveModel, ModelTrait,
        PaginatorTrait, QueryFilter, QuerySelect, TransactionTrait,
    };
    pub use std::sync::Arc;
    pub use tonic::*;
    pub use uuid::Uuid;
}
