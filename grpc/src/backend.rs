pub mod backend {
    tonic::include_proto!("oj.backend");
}
pub use backend::*;

impl From<String> for backend::Token {
    fn from(value: String) -> Self {
        Token { signature: value }
    }
}

impl backend::Paginator {
    pub fn new(session: String) -> Self {
        Self { session }
    }
}

impl From<backend::Paginator> for String {
    fn from(value: Paginator) -> Self {
        value.session
    }
}
