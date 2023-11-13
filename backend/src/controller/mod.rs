pub mod submit;
pub mod token;
pub mod util;

use sea_orm::ActiveValue;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("`{0}`")]
    Database(#[from] sea_orm::error::DbErr),
    #[error("`{0}`")]
    Tonic(#[from] tonic::transport::Error),
    #[error("`{0}`")]
    Transaction(#[from] sea_orm::TransactionError<sea_orm::error::DbErr>),
    #[error("`{0}`")]
    Submit(submit::Error),
    #[error("`{0}`")]
    GrpcReport(#[from] tonic::Status),
    #[error("`{0}`")]
    Internal(&'static str),
}

pub fn to_active_value<C>(option: Option<C>) -> ActiveValue<C>
where
    C: Into<sea_orm::Value>,
{
    match option {
        Some(x) => ActiveValue::Set(x),
        None => ActiveValue::default(),
    }
}
