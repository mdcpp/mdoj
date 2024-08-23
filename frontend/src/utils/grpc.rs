pub use grpc::backend::*;
use tonic::{metadata::MetadataMap, IntoRequest, Request};

use super::frontend_config;

#[cfg(not(feature = "ssr"))]
pub fn new_client() -> tonic_web_wasm_client::Client {
    use tonic_web_wasm_client::Client;
    let config = frontend_config();

    Client::new(config.api_server.clone())
}

#[cfg(feature = "ssr")]
pub fn new_client() -> tonic::transport::Channel {
    use tonic::transport::Endpoint;

    let config = frontend_config();
    Endpoint::new(config.api_server.clone())
        .expect("cannot parse backend url")
        .connect_lazy()
}

pub trait WithToken: Sized {
    /// this will add a optional token to request.
    ///
    /// Will do nothing if token is `None`
    fn with_optional_token(self, token: Option<String>) -> Request<Self>;

    /// this will add token to request.
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
        let mut metadata = MetadataMap::new();
        metadata.insert("token", token);
        #[cfg(feature = "ssr")]
        let metadata = with_xff(metadata);
        *req.metadata_mut() = metadata;
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
fn with_xff(metadata: MetadataMap) -> MetadataMap {
    use actix_web::http::header;
    use leptos::*;
    use leptos_actix::ResponseOptions;

    let mut header_map = metadata.into_headers();
    let options = expect_context::<ResponseOptions>();
    let options = options.0.read();
    let addr = options.headers.get(header::X_FORWARDED_FOR);
    if let Some(addr) = addr {
        header_map.insert(header::X_FORWARDED_FOR, addr.clone());
    }
    MetadataMap::from_headers(header_map)
}
