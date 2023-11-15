use tonic::async_trait;

use self::error::Error;

pub mod auth;
pub mod error;
pub mod macro_tool;
pub mod pagination;

#[async_trait]
pub trait ControllerTrait {
    async fn parse_request<T>(&self, request: tonic::Request<T>) -> Result<(auth::Auth, T), Error>
    where
        T: Send;
}
