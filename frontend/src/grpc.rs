tonic::include_proto!("oj.backend");
pub use announcement_set_client::*;
pub use chat_set_client::*;
pub use contest_set_client::*;
pub use problem_set_client::*;
pub use token_set_client::*;

use crate::config::server_config;
use anyhow::Result;
use tonic_web_wasm_client::Client;

pub async fn new_client() -> Result<Client> {
    let config = server_config().await?;
    Ok(Client::new(config.backend))
}
