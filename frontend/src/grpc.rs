pub use grpc::backend::*;
use leptos::*;
use tonic::{metadata::MetadataMap, IntoRequest, Request};

use crate::config::frontend_config;

#[cfg(not(feature = "ssr"))]
pub fn new_client() -> tonic_web_wasm_client::Client {
    use tonic_web_wasm_client::Client;
    let config = frontend_config();

    Client::new(config.api_server.clone())
}

#[cfg(feature = "ssr")]
pub fn new_client() -> tonic::transport::Channel {
    use tonic::transport::{Channel, Endpoint};

    let config = frontend_config();
    Endpoint::new(config.api_server.clone())
        .expect("cannot parse backend url")
        .connect_lazy()
}

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
        let metadata = req.metadata_mut();
        metadata.insert("token", token);
        #[cfg(feature = "ssr")]
        with_xff(metadata);
        req
    }

    fn with_optional_token(self, token: Option<String>) -> Request<Self> {
        let Some(token) = token else {
            return self.into_request();
        };
        self.with_token(token)
    }
}

#[cfg(feature = "ssr")]
fn with_xff(metadata: &mut MetadataMap) {
    use std::str::FromStr;

    use actix_web::http::header;
    use leptos_actix::ResponseOptions;
    use tonic::metadata::MetadataValue;

    let options = expect_context::<ResponseOptions>();
    let options = options.0.read();
    let addr = options.headers.get(header::X_FORWARDED_FOR);
    if let Some(addr) = addr {
        metadata.insert(
            "x-forwarded-for",
            MetadataValue::from_str(addr.to_str().unwrap()).unwrap(),
        );
    }
}
