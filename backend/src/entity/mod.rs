// FIXME: improve tracing after refactor

//! collection of entity

pub mod announcement;
pub mod chat;
pub mod contest;
pub mod education;
pub mod problem;
pub mod submit;
pub mod testcase;
pub mod token;
pub mod user;
pub mod user_contest;
pub mod util;

use sea_orm::{
    entity::prelude::*, EntityTrait, FromQueryResult, PrimaryKeyTrait, QueryFilter, Select,
};

use crate::util::{auth::Auth, error::Error};
use tonic::async_trait;

use crate::util::auth::RoleLv;
use util::filter::{Filter, ParentalTrait};
use util::paginator::*;
use util::with::*;
