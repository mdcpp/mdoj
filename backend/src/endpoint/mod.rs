pub mod announcement;
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

mod tools {
    pub use crate::entity::util::paginator::Pager;

    pub use crate::util::auth::PermLevel;
    pub use crate::util::auth::Auth;

    // pub use crate::entity::util::paginator::Pager;
    pub use crate::entity::DebugName;
    pub use crate::entity::*;
    pub use crate::grpc::TonicStream;
    pub use crate::init::db::DB;

    pub use crate::util::error::Error;
    // pub use crate::util::pager::*;
    pub use crate::{fill_active_model, fill_exist_active_model, server::Server};
    pub use sea_orm::{
        ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, IntoActiveModel, ModelTrait,
        PaginatorTrait, QueryFilter, QuerySelect, TransactionTrait,
    };
    pub use std::sync::Arc;
    pub use tokio::spawn;
    pub use tokio::try_join;
    pub use tonic::*;
    pub use tracing::instrument;
    pub use uuid::Uuid;
}
