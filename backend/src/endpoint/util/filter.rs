use sea_orm::*;

use super::{auth::Auth, error::Error};

pub trait Filter
where
    Self: EntityTrait,
{
    fn read_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
        Err(Error::Unauthenticated)
    }
    fn write_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
        Err(Error::Unauthenticated)
    }
}

pub trait ParentalFilter {
    fn publish_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
        Err(Error::Unauthenticated)
    }
    fn link_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
        Err(Error::Unauthenticated)
    }
}
