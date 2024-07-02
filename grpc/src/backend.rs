pub mod backend {
    tonic::include_proto!("oj.backend");
}
pub use backend::*;

impl From<String> for backend::Token {
    fn from(value: String) -> Self {
        Token { signature: value }
    }
}
