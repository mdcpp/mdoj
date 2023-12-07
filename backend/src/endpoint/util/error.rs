use crate::report_internal;

#[derive(Debug, thiserror::Error)]
pub enum Error {
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
    #[error("Number too large(or small)")]
    NumberTooLarge,
}

impl From<Error> for tonic::Status {
    fn from(value: Error) -> Self {
        match value {
            Error::PremissionDeny(x) => {
                log::debug!("Client request inaccessible resource, hint: {}", x);
                tonic::Status::permission_denied(x)
            }
            Error::DBErr(x) => report_internal!(error, "{}", x),
            // all argument should be checked before processing,
            // so this error is considered as internal error
            Error::BadArgument(x) => {
                log::debug!("Client sent invaild argument: payload.{}", x);
                tonic::Status::invalid_argument(x)
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
            Error::Unreachable(x) => report_internal!(error, "{}", x),
            Error::NumberTooLarge => tonic::Status::failed_precondition("number too large"),
        }
    }
}
