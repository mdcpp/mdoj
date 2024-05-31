use tonic::Status;

use super::sandbox::Error as SandboxError;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("sandbox error: {0}")]
    Sandbox(#[from] SandboxError),
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
    #[error("invaild secret")]
    InvaildSecret,
    #[error("invaild language uuid")]
    InvaildLanguageUuid,
    #[error("impossible memory requirement")]
    ImpossibleMemoryRequirement,
}

impl From<ClientError> for Status {
    fn from(value: ClientError) -> Self {
        match value {
            ClientError::InvaildSecret => Status::permission_denied("Invaild secret"),
            ClientError::InvaildLanguageUuid => {
                Status::failed_precondition("Invaild language uuid")
            }
            ClientError::ImpossibleMemoryRequirement => {
                Status::failed_precondition("Impossible memory requirement")
            }
        }
    }
}
