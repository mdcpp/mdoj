use thiserror::Error;

use crate::grpc::prelude::JudgeMatchRule;

#[derive(Debug, Error)]
pub enum Error {
    #[error("`{0}`")]
    Database(#[from] sea_orm::error::DbErr),
    #[error("`{0}`")]
    Transaction(#[from] sea_orm::TransactionError<sea_orm::error::DbErr>),
    #[error("`{0}`")]
    Tonic(#[from] tonic::transport::Error),
}

pub struct RouteRequest{
    pub match_rule: JudgeMatchRule,
    pub code: Vec<u8>,
    pub language: String,
}

pub trait Routable{
}



// There are two type of router, single and multiple(hot reload)
// Router is responsible for 
// 1. routing the request to the judger with corresponding language support
// 2. expose the language support to the endpoints
// 3. watch change of the running tasks, notify the endpoints with spsc channel
// 4. health check

// single router:
// very simple router, only one judger, no hot reloadable
// keep in mind don't overflow judger's buffer(256MiB)
// If it's going to overflow, return error to the endpoint

// multiple router:
// multiple judgers, hot reloadable, thick client
