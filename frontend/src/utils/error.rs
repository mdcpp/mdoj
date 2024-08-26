use std::fmt::{self, Display};

use leptos::logging;
use serde::{Deserialize, Serialize};
pub type Result<T, E = Error> = core::result::Result<T, E>;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Error {
    pub kind: ErrorKind,
    pub context: String,
}

impl Error {
    pub fn new(kind: ErrorKind, context: impl Into<String>) -> Self {
        Self {
            kind,
            context: context.into(),
        }
    }
}

impl std::error::Error for Error {}
impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} : {}", self.kind, self.context)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ErrorKind {
    /// api error
    NotFound,
    RateLimit,
    Unauthenticated,
    PermissionDenied,
    OutOfRange,
    ApiNotMatch,

    /// runtime error
    Network,
    Browser,
    Internal,

    /// User error
    MalformedUrl,

    /// Other
    Unimplemented,
}

impl Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorKind::NotFound => write!(f, "Not Found"),
            ErrorKind::RateLimit => write!(f, "Rate limited"),
            ErrorKind::Unauthenticated => write!(f, "Unauthenticated"),
            ErrorKind::PermissionDenied => write!(f, "Permission Denied"),
            ErrorKind::OutOfRange => write!(f, "Out Of Range"),
            ErrorKind::Network => write!(f, "Network Error"),
            ErrorKind::Browser => write!(f, "Browser Error"),
            ErrorKind::Internal => write!(f, "Internal Error"),
            ErrorKind::MalformedUrl => write!(f, "Malformed Url"),
            ErrorKind::ApiNotMatch => {
                write!(f, "Cannot call API, please check API version")
            }
            ErrorKind::Unimplemented => write!(f, "Unimplemented right now"),
        }
    }
}

impl From<leptos::error::Error> for Error {
    fn from(value: leptos::error::Error) -> Self {
        value
            .downcast_ref::<Error>()
            .expect("Type of error should be `ErrorKind`")
            .clone()
    }
}

impl From<tonic::Status> for Error {
    fn from(value: tonic::Status) -> Self {
        use tonic::Code;

        let kind = match value.code() {
            Code::NotFound => ErrorKind::NotFound,
            Code::Unauthenticated => ErrorKind::Unauthenticated,
            Code::PermissionDenied => ErrorKind::PermissionDenied,
            Code::DeadlineExceeded | Code::Unavailable => ErrorKind::Network,
            Code::OutOfRange => ErrorKind::OutOfRange,
            // this happened when grpc cannot find the rpc
            Code::Unimplemented => ErrorKind::ApiNotMatch,
            code => {
                logging::error!("{code}");
                ErrorKind::Internal
            }
        };
        let context = value.message().to_owned();

        Self { kind, context }
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self {
            kind: ErrorKind::Internal,
            context: value.to_string(),
        }
    }
}

impl From<leptos_router::ParamsError> for Error {
    fn from(value: leptos_router::ParamsError) -> Self {
        let context = match value {
            leptos_router::ParamsError::MissingParam(err) => err,
            leptos_router::ParamsError::Params(err) => err.to_string(),
        };
        Self {
            kind: ErrorKind::MalformedUrl,
            context,
        }
    }
}

#[cfg(feature = "ssr")]
impl From<tonic::transport::Error> for Error {
    fn from(value: tonic::transport::Error) -> Self {
        Self {
            kind: ErrorKind::Internal,
            context: value.to_string(),
        }
    }
}

pub trait Context {
    type Output;
    fn context(self, c: impl AsRef<str>) -> Self::Output;
}

impl<E> Context for E
where
    E: Into<Error>,
{
    type Output = Error;

    fn context(self, c: impl AsRef<str>) -> Self::Output {
        let mut err: Error = self.into();
        err.context.push_str("\n  >");
        err.context.push_str(c.as_ref());
        err
    }
}

impl<T, E> Context for Result<T, E>
where
    E: Into<Error>,
{
    type Output = Result<T>;

    fn context(self, c: impl AsRef<str>) -> Self::Output {
        self.map_err(|err| {
            let mut err: Error = err.into();
            err.context.push_str("\n  >");
            err.context.push_str(c.as_ref());
            err
        })
    }
}
