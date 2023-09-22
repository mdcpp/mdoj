pub mod submit;
pub mod token;
use migration::Token;
use sea_orm::ActiveValue;
use thiserror::Error;

use self::token::TokenController;

#[derive(Debug, Error)]
pub enum Error {
    #[error("`{0}`")]
    Database(#[from] sea_orm::error::DbErr),
    #[error("`{0}`")]
    Transaction(#[from] sea_orm::TransactionError<sea_orm::error::DbErr>),
    #[error("`{0}`")]
    Tonic(#[from] tonic::transport::Error),
    #[error("The upstream server has error that can possibly be solved by retry")]
    ShouldRetry,
    #[error("All `{0}` service was unavailable")]
    Unavailable(String),
    #[error("primary key not found for `{0}`")]
    NotFound(&'static str),
    #[error("Database corrupted")]
    Corrupted,
}

impl Error {
    pub fn should_retry(&self) -> bool {
        match self {
            Error::ShouldRetry => true,
            _ => false,
        }
    }
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
