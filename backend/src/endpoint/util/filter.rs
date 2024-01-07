use sea_orm::*;

use super::{auth::Auth, error::Error};

/// filter for Entity r/w
pub trait Filter
where
    Self: EntityTrait,
{
    fn read_filter<S: QueryFilter + Send>(_: S, _: &Auth) -> Result<S, Error> {
        Err(Error::Unauthenticated)
    }
    fn write_filter<S: QueryFilter + Send>(_: S, _: &Auth) -> Result<S, Error> {
        Err(Error::Unauthenticated)
    }
}

/// filter related to across Entity relation
pub trait ParentalFilter {
    fn publish_filter<S: QueryFilter + Send>(_: S, _: &Auth) -> Result<S, Error> {
        Err(Error::Unauthenticated)
    }
    fn link_filter<S: QueryFilter + Send>(_: S, _: &Auth) -> Result<S, Error> {
        Err(Error::Unauthenticated)
    }
}
