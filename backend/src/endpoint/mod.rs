pub mod chat;
pub mod contest;
pub mod education;
pub mod imgur;
pub mod playground;
pub mod problem;
pub mod submit;
pub mod testcase;
pub mod token;
pub mod user;
pub mod util;

pub mod tools {
    pub use super::util::auth::Auth;
    pub use super::util::error::Error;
    pub use crate::grpc::TonicStream;
    pub use crate::init::db::DB;
    pub use tracing::instrument;
}
pub mod endpoints {
    pub use super::util::{
        filter::{Filter, ParentalFilter},
        pager::*,
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
