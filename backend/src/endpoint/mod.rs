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
mod problem;
mod submit;
mod testcase;
mod token;
mod user;

use crate::NonZeroU32;
use grpc::backend::{Id, Order, *};
use sea_orm::{*, Value};
use std::ops::Deref;
use tonic::*;
use tracing::*;
use uuid::Uuid;

use crate::entity::util::{filter::*, order::*};
use crate::util::with::*;
use crate::util::{auth::RoleLv, bound::BoundCheck, duplicate::*, error::Error, time::*};
use crate::{fill_active_model, fill_exist_active_model, server::ArcServer, TonicStream};
use tracing::{Instrument, Level};
