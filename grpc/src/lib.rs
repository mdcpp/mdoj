#[cfg(feature = "wkt")]
pub mod backend {
    tonic::include_proto!("oj.backend");
}

#[cfg(feature = "backend")]
#[cfg(not(feature = "wkt"))]
pub mod backend {
    tonic::include_proto!("oj.backend");
}

#[cfg(feature = "backend")]
pub mod judger {
    tonic::include_proto!("oj.judger");
}
