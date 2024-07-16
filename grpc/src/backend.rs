#[allow(clippy::all, non_local_definitions)]
mod backend {
    tonic::include_proto!("oj.backend");
}

pub use backend::*;
