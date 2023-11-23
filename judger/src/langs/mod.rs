use thiserror::Error;

use crate::{grpc::proto::prelude::JudgerCode, sandbox};

pub mod artifact;
pub mod spec;

pub mod prelude {
    pub use super::artifact::*;
    pub use super::{Error, InternalError, RequestError};
}

#[derive(Error, Debug)]
pub enum InternalError {
    #[error("`{0}`")]
    Serde(#[from] toml::de::Error),
    #[error("Language exstension \"spec.toml\" malformated")]
    FileMalFormat,
    #[error("Language \"spec.toml\" does not exist")]
    FileNotExist,
    #[error("`{0}`")]
    JailError(sandbox::Error),
}

#[derive(Error, Debug)]
pub enum RequestError {
    #[error("Language not found")]
    LangNotFound(String),
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("Internal Error: `{0}`")]
    Internal(#[from] InternalError),
    #[error("Bad Request: `{0}`")]
    BadRequest(#[from] RequestError),
    #[error("Report the result to client")]
    Report(JudgerCode),
}

impl From<sandbox::Error> for Error {
    fn from(value: sandbox::Error) -> Self {
        match value {
            sandbox::Error::ImpossibleResource
            | sandbox::Error::Stall
            | sandbox::Error::CapturedPipe => Error::Report(JudgerCode::Re),
            sandbox::Error::IO(_)
            | sandbox::Error::ControlGroup(_)
            | sandbox::Error::Libc(_)
            | sandbox::Error::CGroup => Error::Internal(InternalError::JailError(value)),
            sandbox::Error::BufferFull => Error::Report(JudgerCode::Ole),
        }
    }
}
