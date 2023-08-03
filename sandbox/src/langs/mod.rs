use thiserror::Error;

use crate::{
    grpc::proto::{self, prelude::JudgeResultState},
    jail,
};

pub mod artifact;
pub mod spec;

#[derive(Error, Debug)]
pub enum InternalError {
    #[error("`{0}`")]
    Serde(#[from] toml::de::Error),
    #[error("Language exstension \"spec.toml\" malformated")]
    FileMalFormat,
    #[error("Language \"spec.toml\" does not exist")]
    FileNotExist,
    #[error("`{0}`")]
    JailError(jail::Error),
}

#[derive(Error, Debug)]
pub enum RequestError {
    #[error("Language not found")]
    LangNotFound,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("Internal Error: `{0}`")]
    Internal(#[from] InternalError),
    #[error("Bad Request: `{0}`")]
    BadRequest(#[from] RequestError),
    #[error("Report the result to client")]
    Report(JudgeResultState),
}

impl From<jail::Error> for Error {
    fn from(value: jail::Error) -> Self {
        match value {
            jail::Error::ImpossibleResource | jail::Error::Stall | jail::Error::CapturedPipe => {
                Error::Report(JudgeResultState::Re)
            }
            jail::Error::IO(_)
            | jail::Error::ControlGroup(_)
            | jail::Error::Libc(_)
            | jail::Error::CGroup => Error::Internal(InternalError::JailError(value)),
            jail::Error::BufferFull => Error::Report(JudgeResultState::Ole),
        }
    }
}
