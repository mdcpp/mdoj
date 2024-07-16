use sea_orm::entity::prelude::*;

use crate::util::{auth::Auth, error::Error};
use tonic::async_trait;

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
    /// shortcut for empty `find` with read filter applied
    fn read_find(auth: &Auth) -> Result<Select<Self>, Error> {
        Self::read_filter(Self::find(), auth)
    }
    /// read filter
    fn read_filter<S: QueryFilter + Send>(_: S, _: &Auth) -> Result<S, Error> {
        Err(Error::Unauthenticated)
    }
    /// write filter
    fn write_filter<S: QueryFilter + Send>(_: S, _: &Auth) -> Result<S, Error> {
        Err(Error::Unauthenticated)
    }
    /// shortcut for empty `find_by_id` with read filter applied
    fn read_by_id<T>(id: T, auth: &Auth) -> Result<Select<Self>, Error>
    where
        T: Into<<Self::PrimaryKey as PrimaryKeyTrait>::ValueType>,
    {
        Self::read_filter(Self::find_by_id(id), auth)
    }
    /// shortcut for empty `find_by_id` with write filter applied
    fn write_by_id<T>(id: T, auth: &Auth) -> Result<Select<Self>, Error>
    where
        T: Into<<Self::PrimaryKey as PrimaryKeyTrait>::ValueType>,
    {
        Self::write_filter(Self::find_by_id(id), auth)
    }
    fn writable(model: &Self::Model, auth: &Auth) -> bool {
        false
    }
}
