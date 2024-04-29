pub use grpc::backend::*;

use crate::{config::server_config, error::*};

cfg_if::cfg_if! {if #[cfg(feature = "ssr")] {
    use tonic::transport::{Endpoint,Channel};
    pub async fn new_client() -> Result<Channel> {
        let config = server_config().await?;
        Ok(Endpoint::new(config.backend)?.connect().await?)
    }
} else {
    use tonic_web_wasm_client::Client;
    pub async fn new_client() -> Result<Client> {
        let config = server_config().await?;
        Ok(Client::new(config.backend))
    }
}}
