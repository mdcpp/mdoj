//! collection of endpoint implementation from high level
//!
//! We don't use helper or some extra trait
//!
//! It's a decision to avoid coupling between each endpoint
mod announcement;
mod chat;
mod contest;
mod education;
mod imgur;
mod playground;
mod problem;
mod submit;
mod testcase;
mod token;
mod user;

mod tools {
    pub const SHORT_ART_SIZE: usize = 128;
    pub const LONG_ART_SIZE: usize = 65536;
    pub use crate::entity::util::paginator::{Pager, Remain};

    pub use crate::grpc::TonicStream;
    pub use crate::NonZeroU32;
    pub use std::ops::Deref;

    pub use crate::util::auth::RoleLv;

    pub use crate::{bound, entity::*};

    pub use crate::util::error::Error;
    pub use crate::{
        check_exist_length, check_length, fill_active_model, fill_exist_active_model,
        server::Server,
    };
    pub use sea_orm::{
        ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, IntoActiveModel, ModelTrait,
        PaginatorTrait, QueryFilter, QuerySelect, TransactionTrait,
    };
    pub use std::sync::Arc;
    pub use tokio::try_join;
    pub use tonic::*;
    pub use tracing::instrument;
    pub use uuid::Uuid;
    pub fn split_rev(raw: i64) -> (bool, u64) {
        (raw < 0, raw.abs().try_into().unwrap())
    }
}
