use super::sandbox::Error as SandboxError;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("sandbox error: {0}")]
    Sandbox(#[from] SandboxError),
}
