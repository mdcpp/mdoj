pub use grpc::backend::*;
use tonic::{IntoRequest, Request};

use crate::{config::frontend_config, error::*};

cfg_if::cfg_if! {if #[cfg(feature = "ssr")] {
    use tonic::transport::{Endpoint,Channel};
    pub async fn new_client() -> Result<Channel> {
        let config = frontend_config().await?;
        Ok(Endpoint::new(config.api_server)?.connect_lazy())
    }
} else {
    use tonic_web_wasm_client::Client;
    pub async fn new_client() -> Result<Client> {
        let config = frontend_config().await?;

        Ok(Client::new(config.api_server))
    }
}}

pub trait WithToken: Sized {
    /// this will try to add token to request.
    ///
    /// Will return error if token is not exist
    fn with_optional_token(self, token: Option<String>) -> Request<Self>;

    /// this will try to add token to request.
    ///
    /// If token is not exist, it will just ignore error and return request without token
    fn with_token(self, token: String) -> Request<Self>;
}

impl<T> WithToken for T
where
    T: IntoRequest<T>,
{
    fn with_token(self, token: String) -> Request<Self> {
        let mut req = self.into_request();
        let Ok(token) = token.parse() else {
            return req;
        };
        req.metadata_mut().insert("token", token);
        req
    }

    fn with_optional_token(self, token: Option<String>) -> Request<Self> {
        let Some(token) = token else {
            return self.into_request();
        };
        self.with_token(token)
    }
}
