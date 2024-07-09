#[allow(clippy::all, non_local_definitions)]
mod backend {
    tonic::include_proto!("oj.backend");
}

pub use backend::*;

impl From<i32> for backend::Id {
    fn from(value: i32) -> Self {
        backend::Id { id: value }
    }
}

impl From<backend::Id> for i32 {
    fn from(value: Id) -> Self {
        value.id
    }
}
