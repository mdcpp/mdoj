use tokio::sync::broadcast::error;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    CgroupError(#[from] cgroups_rs::error::Error),
    #[error("insufficient `{0}`")]
    Insufficient(&'static str),
    // #[error("Out Of `{0}`")]
    // OutOfResource(crate::sandbox::ResourceKind),
    #[error("io error")]
    IoError(#[from] std::io::Error),
    #[error("invaild tarball: `{0}`")]
    InvalidTarball(&'static str),
}
