use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Upstream: `{0}`")]
    Upstream(#[from] crate::controller::Error),
    #[error("Premission deny: `{0}`")]
    PremissionDeny(&'static str),
    #[error("seaorm error: `{0}`")]
    DBErr(#[from] sea_orm::DbErr),
    #[error("Downstream: `{0}`")]
    BadArgument(&'static str),
    #[error("Not in payload: `{0}`")]
    NotInPayload(&'static str),
    #[error("Unauthenticated")]
    Unauthenticated,
    #[error("Not in database: `{0}`")]
    NotInDB(&'static str),
}

impl Into<tonic::Status> for Error {
    fn into(self) -> tonic::Status {
        match self {
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
        }
    }
}
