use thiserror::Error;


#[derive(Debug, Error)]
pub enum Error {
    #[error("Upstream: `{0}`")]
    Upstream(#[from] crate::controller::Error),
    #[error("Premission deny: `{0}`")]
    PremissionDeny(&'static str),
    #[error("seaorm error: `{0}`")]
    DBErr(#[from] sea_orm::DbErr),
    #[error("payload.`{0}` is not a vaild argument")]
    BadArgument(&'static str),
    #[error("Not in payload: `{0}`")]
    NotInPayload(&'static str),
    #[error("Unauthenticated")]
    Unauthenticated,
    #[error("Not in database: `{0}`")]
    NotInDB(&'static str),
    #[error("Invaild Pager`{0}`")]
    PaginationError(&'static str),
    #[error("Invaild request_id")]
    InvaildUUID(#[from] uuid::Error),
    #[error("Function should be unreachable!")]
    Unreachable(&'static str),
}

impl From<Error> for tonic::Status {
    fn from(value: Error) -> Self {
        match value {
            Error::Upstream(x) => {
                log::error!("{}", x);
                #[cfg(feature = "unsecured-log")]
                return tonic::Status::internal(format!("{}", x));
                tonic::Status::unavailable("")
            }
            Error::PremissionDeny(x) => {
                log::debug!("Client request inaccessible resource, hint: {}", x);
                tonic::Status::permission_denied(x)
            }
            Error::DBErr(x) => {
                log::error!("{}", x);
                #[cfg(feature = "unsecured-log")]
                return tonic::Status::internal(format!("{}", x));
                tonic::Status::unavailable("")
            }
            // all argument should be checked before processing,
            // so this error is considered as internal error
            Error::BadArgument(x) => {
                log::warn!("Client sent invaild argument: payload.{}", x);
                #[cfg(feature = "unsecured-log")]
                return tonic::Status::invalid_argument(format!("Bad Argument {}", x));
                tonic::Status::invalid_argument("")
            }
            Error::NotInPayload(x) => {
                log::trace!("{} is not found in client payload", x);
                tonic::Status::invalid_argument(format!("payload.{} is not found", x))
            }
            Error::Unauthenticated => {
                log::debug!("Client sent invaild or no token");
                tonic::Status::unauthenticated("")
            }
            Error::NotInDB(x) => {
                log::debug!("{} is not found in database", x);
                tonic::Status::not_found("")
            }
            Error::PaginationError(x) => {
                log::debug!("{} is not a vaild pager", x);
                tonic::Status::failed_precondition(x)
            }
            Error::InvaildUUID(err) => {
                log::trace!("Fail parsing request_id: {}", err);
                tonic::Status::invalid_argument(
                    "Invaild request_id(should be a client generated UUIDv4)",
                )
            }
            Error::Unreachable(x) => {
                log::error!("Function should be unreachable: {}", x);
                #[cfg(feature = "unsecured-log")]
                return tonic::Status::internal(format!("Function should be unreachable: {}", x));
                tonic::Status::aborted("")
            }
        }
    }
}
