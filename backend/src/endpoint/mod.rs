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
    /// longest allowed size for short string
    pub const SHORT_ART_SIZE: usize = 128;
    /// longest allowed size for long string
    pub const LONG_ART_SIZE: usize = 65536;
    pub use crate::entity::util::paginator::{Pager, Remain};

    pub use crate::grpc::TonicStream;
    pub use crate::NonZeroU32;
    pub use std::ops::Deref;

    pub use crate::entity::util::filter::*;
    pub use crate::util::{
        auth::RoleLv,
        error::{atomic_fail, Error},
    };
    pub use crate::{
        check_exist_length, check_length, fill_active_model, fill_exist_active_model,
        parse_pager_param, server::Server,
    };
    pub use sea_orm::*;
    pub use std::sync::Arc;
    pub use tonic::*;
    pub use tracing::*;
    pub use uuid::Uuid;
}

// FIXME: currently we report transaction error as internal error,
// but we should report it as bad request, or even a give it an retry logic
