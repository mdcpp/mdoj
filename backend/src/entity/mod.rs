use sea_orm::{
    entity::prelude::*, EntityTrait, FromQueryResult, PrimaryKeyTrait, QueryFilter, Select,
};

use sea_orm::ColumnTrait;
use util::paginator::{ColPager, PagerSortSource};

pub mod announcement;
pub mod chat;
pub mod contest;
pub mod education;
pub mod problem;
pub mod submit;
pub mod test;
pub mod token;
pub mod user;
pub mod user_contest;
pub mod util;

use crate::util::{auth::Auth, error::Error};
use tonic::async_trait;

use util::paginator::{PagerReflect, PagerSource, PkPager};

pub trait DebugName {
    const DEBUG_NAME: &'static str = "TEMPLATE_DEBUG_NAME";
}

/// Parental filter are useful when list by parent, mainly because we don't want to list all entity
///
/// For example, on page of problem, we only want to show public problem(even user have joined contest)
#[async_trait]
pub trait ParentalTrait<M> {
    async fn related_read_by_id(auth: &Auth, id: i32, db: &DatabaseConnection) -> Result<M, Error>;
}

/// filter for Entity r/w
pub trait Filter
where
    Self: EntityTrait,
{
    fn read_find(auth: &Auth) -> Result<Select<Self>, Error> {
        Self::read_filter(Self::find(), auth)
    }
    fn read_filter<S: QueryFilter + Send>(_: S, _: &Auth) -> Result<S, Error> {
        Err(Error::Unauthenticated)
    }
    fn write_filter<S: QueryFilter + Send>(_: S, _: &Auth) -> Result<S, Error> {
        Err(Error::Unauthenticated)
    }
    fn read_by_id<T>(id: T, auth: &Auth) -> Result<Select<Self>, Error>
    where
        T: Into<<Self::PrimaryKey as PrimaryKeyTrait>::ValueType>,
    {
        Self::read_filter(Self::find_by_id(id), auth)
    }
    fn write_by_id<T>(id: T, auth: &Auth) -> Result<Select<Self>, Error>
    where
        T: Into<<Self::PrimaryKey as PrimaryKeyTrait>::ValueType>,
    {
        Self::write_filter(Self::find_by_id(id), auth)
    }
}
