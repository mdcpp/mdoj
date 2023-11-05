use tonic::async_trait;

use crate::Server;

use self::{auth::Auth, error::Error};

pub mod auth;
pub mod error;

#[async_trait]
pub trait ControllerTrait {
    async fn parse_request<T>(&self, request: tonic::Request<T>) -> Result<(auth::Auth, T), Error>
    where
        T: Send;
}
