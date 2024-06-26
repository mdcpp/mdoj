#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    CgroupError(#[from] cgroups_rs::error::Error),
    #[error("io error")]
    IoError(#[from] std::io::Error),
}
