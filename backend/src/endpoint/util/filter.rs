use sea_orm::*;

use super::{auth::Auth, error::Error};

#[tonic::async_trait]
pub trait Filter
where
    Self: EntityTrait,
{
    async fn read_filter(query: Select<Self>, auth: &Auth) -> Result<Select<Self>, Error>;
    async fn write_filter(query: Select<Self>, auth: &Auth) -> Result<Select<Self>, Error>;
}
