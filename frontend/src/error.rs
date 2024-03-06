use leptos::{error::Error, ServerFnError};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tonic::{Code, Status};

pub type Result<T, E = ErrorKind> = core::result::Result<T, E>;

#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize)]
pub enum ErrorKind {
    #[error("The resource you request it not exist")]
    NotFound,
    #[error("Oh wait wait! Slow down")]
    RateLimit,
    #[error("You need login first")]
    LoginRequire,
    #[error("Permission denied")]
    PermissionDenied,
    #[error("Network error, please check your connection")]
    Network,
    #[error("Please use supported browser")]
    Browser,
    #[error("Something went wrong: '{0}'")]
    ServerError(#[from] ServerErrorKind),
}

#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize)]
pub enum ServerErrorKind {
    #[error("Io fail")]
    IoError,
    #[error("Server function error")]
    ServerFn,
    #[error("Please check the version of backend is compatible with frontend")]
    InvalidValue,
    #[error("Unknown error")]
    Unknown,
}

impl From<Error> for ErrorKind {
    fn from(value: Error) -> Self {
        value
            .downcast_ref::<ErrorKind>()
            .expect("Type of error should be `ErrorKind`")
            .clone()
    }
}

impl From<Status> for ErrorKind {
    fn from(value: Status) -> Self {
        match value.code() {
            Code::NotFound => ErrorKind::NotFound,
            Code::Unauthenticated => ErrorKind::LoginRequire,
            Code::PermissionDenied => ErrorKind::PermissionDenied,
            Code::DeadlineExceeded | Code::Unavailable => ErrorKind::Network,
            _ => ErrorKind::ServerError(ServerErrorKind::Unknown),
        }
    }
}

impl From<ServerFnError> for ErrorKind {
    fn from(_: ServerFnError) -> Self {
        ErrorKind::ServerError(ServerErrorKind::ServerFn)
    }
}

impl From<std::io::Error> for ErrorKind {
    fn from(_: std::io::Error) -> Self {
        ErrorKind::ServerError(ServerErrorKind::IoError)
    }
}

#[cfg(feature = "ssr")]
impl From<tonic::transport::Error> for ErrorKind {
    fn from(_: tonic::transport::Error) -> Self {
        ErrorKind::ServerError(ServerErrorKind::Unknown)
    }
}
