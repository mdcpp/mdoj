#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    CgroupError(#[from] cgroups_rs::error::Error),
    #[error("too few memory")]
    LowMemory,
    #[error("queue is full")]
    QueueFull,
}
