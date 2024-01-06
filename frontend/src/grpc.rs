use tonic_web_wasm_client::Client;

tonic::include_proto!("oj.backend");
pub use token_set_client::*;

pub fn new_client() -> Client {
    Client::new("http://0.0.0.0:8081".into())
}
