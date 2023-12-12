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
    #[error("Buffer `{0}` too large")]
    BufferTooLarge(&'static str),
    #[error("Already exist")]
    AlreadyExist(&'static str),
}

impl From<Error> for tonic::Status {
    fn from(value: Error) -> Self {
        match value {
            Error::PremissionDeny(x) => {
                tracing::debug!(hint = x, "premission_invaild");
                tonic::Status::permission_denied(x)
            }
            Error::DBErr(x) => report_internal!(error, "{}", x),
            Error::BadArgument(x) => {
                tracing::trace!(miss_type = x, "argument_invaild");
                tonic::Status::invalid_argument(x)
            }
            Error::NotInPayload(x) => {
                tracing::trace!(miss_type = x, "argument_missing");
                tonic::Status::invalid_argument(format!("payload.{} is not found", x))
            }
            Error::Unauthenticated => {
                tracing::trace!("Client sent invaild or no token");
                tonic::Status::unauthenticated("")
            }
            Error::NotInDB(x) => {
                tracing::trace!(entity = x, "database_notfound");
                tonic::Status::not_found("")
            }
            Error::PaginationError(x) => {
                tracing::debug!(hint = x, "pager_invaild");
                tonic::Status::failed_precondition(x)
            }
            Error::InvaildUUID(err) => {
                tracing::trace!(reason=?err,"requestid_invaild");
                tonic::Status::invalid_argument(
                    "Invaild request_id(should be a client generated UUIDv4)",
                )
            }
            Error::Unreachable(x) => report_internal!(error, "{}", x),
            Error::NumberTooLarge => tonic::Status::invalid_argument("number too large"),
            Error::BufferTooLarge(x) => {
                tonic::Status::invalid_argument(format!("buffer {} too large", x))
            }
            Error::AlreadyExist(x) => {
                tracing::trace!(hint = x, "entity_exist");
                tonic::Status::already_exists(x)
            }
        }
    }
}
