use tonic::Status;
use crate::report_internal;

use super::auth::RoleLv;

/// Centralized Error for endpoint, usually calling with `Into::into()`
/// to tramsform it into `Status` immediately
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Permission deny: `{0}`")]
    PermissionDeny(&'static str),
    #[error("seaorm error: `{0}`")]
    DBErr(#[from] sea_orm::DbErr),
    #[error("payload.`{0}` is not a vaild argument")]
    BadArgument(&'static str),
    #[error("Not in payload: `{0}`")]
    NotInPayload(&'static str),
    #[error("Unauthenticated")]
    Unauthenticated,
    #[error("Not in database: ")]
    NotInDB,
    #[error("Not in database(out of range):")]
    NotInDBList,
    #[error("Invaild request_id")]
    InvaildUUID(#[from] uuid::Error),
    #[error("Function should be unreachable!")]
    Unreachable(&'static str),
    #[error("Number too large(or small)")]
    NumberTooLarge,
    #[error("Buffer `{0}` too large")]
    BufferTooLarge(&'static str),
    #[error("Already exist")]
    AlreadyExist(String),
    #[error("You need to own `{0}` to add thing onto it")]
    UnownedAdd(&'static str),
    #[error("require permission `{0}`")]
    RequirePermission(RoleLv),
    #[error("rate limit reached")]
    RateLimit(&'static str),
    #[error("`{0}`")]
    PassThrough(Status),
}

impl From<Error> for Status {
    fn from(value: Error) -> Self {
        match value {
            Error::PermissionDeny(x) => {
                tracing::debug!(hint = x, "permission_invaild");
                Status::permission_denied(x)
            }
            Error::DBErr(x) => report_internal!(error, "{}", x),
            Error::BadArgument(x) => {
                tracing::trace!(miss_type = x, "argument_invaild");
                Status::invalid_argument(x)
            }
            Error::NotInPayload(x) => {
                tracing::trace!(miss_type = x, "argument_missing");
                Status::invalid_argument(format!("payload.{} is not found", x))
            }
            Error::Unauthenticated => {
                tracing::trace!("Client sent invaild or no token");
                Status::unauthenticated("")
            }
            Error::NotInDB => {
                tracing::trace!("database_notfound");
                Status::not_found("")
            }
            Error::NotInDBList => {
                tracing::trace!("database_notfound");
                Status::out_of_range("")
            }
            Error::InvaildUUID(err) => {
                tracing::trace!(reason=?err,"requestid_invaild");
                Status::invalid_argument("Invaild request_id(should be a client generated UUIDv4)")
            }
            Error::Unreachable(x) => report_internal!(error, "{}", x),
            Error::NumberTooLarge => Status::invalid_argument("number too large"),
            Error::BufferTooLarge(x) => Status::invalid_argument(format!("{} too large", x)),
            Error::AlreadyExist(x) => {
                tracing::trace!(username = x, "entity_exist");
                Status::already_exists(format!("{} already exist", x))
            }
            Error::UnownedAdd(x) => {
                tracing::trace!(hint = x, "add_fail");
                Status::failed_precondition(format!("You need to own {} to add thing onto it", x))
            }
            Error::RequirePermission(x) => {
                Status::permission_denied(format!("require permission {}", x))
            }
            Error::RateLimit(x) => {
                tracing::warn!(traffic = x, "rate_limit");
                Status::resource_exhausted("rate limit reached!")
            }
            Error::PassThrough(x) => x,
        }
    }
}

pub fn atomic_fail(err: sea_orm::DbErr) -> Status {
    match err {
        sea_orm::DbErr::RecordNotUpdated => Error::NotInDB.into(),
        _ => Error::DBErr(err).into(),
    }
}
