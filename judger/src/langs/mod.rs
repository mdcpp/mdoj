use thiserror::Error;

use crate::sandbox;

pub mod artifact;
pub mod spec;

pub mod prelude {
    pub use super::artifact::*;
}

// Error incur from server setup
#[derive(Error, Debug)]
pub enum InitError {
    #[error("`{0}`")]
    Serde(#[from] toml::de::Error),
    #[error("Language exstension \"spec.toml\" malformated")]
    FileMalFormat,
    #[error("Language \"spec.toml\" does not exist")]
    FileNotExist,
    #[error("`{0}`")]
    Sandbox(#[from] sandbox::Error),
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("Language not found")]
    LangNotFound,
    #[error("`{0}`")]
    Sandbox(#[from] sandbox::Error),
}

impl From<Error> for tonic::Status {
    fn from(value: Error) -> Self {
        match value {
            Error::LangNotFound => tonic::Status::failed_precondition("lang not found"),
            _ => tonic::Status::internal(value.to_string()),
        }
    }
}

// impl From<sandbox::Error> for Error {
//     fn from(value: sandbox::Error) -> Self {
//         match value {
//             sandbox::Error::ImpossibleResource
//             | sandbox::Error::Stall
//             | sandbox::Error::CapturedPipe => Error::Report(JudgerCode::Re),
//             sandbox::Error::IO(_)
//             | sandbox::Error::ControlGroup(_)
//             | sandbox::Error::Libc(_)
//             | sandbox::Error::CGroup => Error::Internal(InternalError::JailError(value)),
//             sandbox::Error::BufferFull => Error::Report(JudgerCode::Ole),
//         }
//     }
// }
