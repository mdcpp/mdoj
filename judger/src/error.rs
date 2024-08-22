use tonic::Status;

use super::sandbox::Error as SandboxError;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("sandbox error: {0}")]
    Sandbox(#[from] SandboxError),
    /// the program is running on a 32-bit platform,
    /// and have an object reached [`u32::MAX`]
    #[error("32 bit problem")]
    Platform,
}

impl From<Error> for Status {
    fn from(value: Error) -> Self {
        log::error!("{:?}", value);
        Status::internal("internal error: unknown")
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ClientError {
    #[error("invalid secret")]
    InvalidSecret,
    #[error("invalid language uuid")]
    InvalidLanguageUuid,
    #[error("impossible memory requirement")]
    ImpossibleMemoryRequirement,
}

impl From<ClientError> for Status {
    fn from(value: ClientError) -> Self {
        match value {
            ClientError::InvalidSecret => Status::permission_denied("Invalid secret"),
            ClientError::InvalidLanguageUuid => {
                Status::failed_precondition("Invalid language uuid")
            }
            ClientError::ImpossibleMemoryRequirement => {
                Status::failed_precondition("Impossible memory requirement")
            }
        }
    }
}
