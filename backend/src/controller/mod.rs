pub mod problem;
pub mod token;
pub mod user;
pub mod util;
use thiserror::Error;

pub struct ControllerCluster {}

#[derive(Debug, Error)]
pub enum Error {
    #[error("`{0}`")]
    Database(#[from] sea_orm::error::DbErr),
    #[error("`{0}`")]
    Transaction(#[from] sea_orm::TransactionError<sea_orm::error::DbErr>),
    #[error("`{0}`")]
    Tomic(#[from]tonic::transport::Error),
}
