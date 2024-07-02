#[cfg(feature = "backend")]
#[allow(clippy::all, non_local_definitions)]
pub mod backend {
    tonic::include_proto!("oj.backend");
}

#[cfg(feature = "judger")]
#[allow(clippy::all, non_local_definitions)]
pub mod judger {
    tonic::include_proto!("oj.judger");
}
