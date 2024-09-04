use crate::controller::{imgur as image, judger, token};
use crate::report_internal;
use tonic::Status;

use super::auth::RoleLv;

pub type Result<T> = std::result::Result<T, Error>;

/// Centralized Error for endpoint, usually calling with `Into::into()`
/// to tramsform it into `Status` immediately
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Permission deny: `{0}`")]
    PermissionDeny(&'static str),
    #[error("seaorm error: `{0}`")]
    DBErr(sea_orm::DbErr),
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
    #[error("`{0}` Already exist")]
    AlreadyExist(&'static str),
    #[error("require permission `{0}`")]
    RequirePermission(RoleLv),
    #[error("rate limit reached")]
    RateLimit(&'static str),
    #[error("image error: `{0}`")]
    Image(#[from] image::Error),
    #[error("judger error: `{0}`")]
    Judger(#[from] judger::Error),
    #[error("token error: `{0}`")]
    Token(#[from] token::Error),
    #[error("retry later")]
    Retry,
}

impl From<sea_orm::DbErr> for Error {
    fn from(value: sea_orm::DbErr) -> Self {
        match value {
            sea_orm::DbErr::RecordNotUpdated => Error::NotInDB,
            _ => Error::DBErr(value),
        }
    }
}

impl From<Error> for Status {
    fn from(value: Error) -> Self {
        match value {
            Error::PermissionDeny(x) => {
                tracing::debug!(hint = x, "permission_invalid");
                Status::permission_denied(x)
            }
            Error::DBErr(x) => report_internal!(error, "{}", x),
            Error::BadArgument(x) => {
                tracing::trace!(miss_type = x, "argument_invalid");
                Status::invalid_argument(x)
            }
            Error::NotInPayload(x) => {
                tracing::trace!(miss_type = x, "argument_missing");
                Status::invalid_argument(format!("payload.{} is not found", x))
            }
            Error::Unauthenticated => {
                tracing::trace!("Client sent invalid or no token");
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
            Error::InvaildUUID(_) => {
                Status::invalid_argument("Invaild request_id(should be a client generated UUIDv4)")
            }
            Error::Unreachable(x) => report_internal!(error, "{}", x),
            Error::NumberTooLarge => Status::invalid_argument("number too large"),
            // Error::BufferTooLarge(x) => Status::invalid_argument(format!("{} too large", x)),
            Error::AlreadyExist(x) => Status::already_exists(format!("{} already exist", x)),
            Error::RequirePermission(x) => {
                Status::permission_denied(format!("require permission {}", x))
            }
            Error::RateLimit(x) => {
                tracing::warn!(traffic = x, "rate_limit");
                Status::resource_exhausted("rate limit reached!")
            }
            Error::Image(x) => report_internal!(error, "{}", x),
            Error::Judger(x) => x.into(),
            Error::Token(x) => x.into(),
            Error::Retry => Status::aborted("Should retry"),
        }
    }
}

#[cfg(not(feature = "insecure-print"))]

pub mod insecure_print {
    use super::*;
    use opentelemetry::trace::{SpanId, TraceContextExt, TraceId};
    use tracing_opentelemetry::OpenTelemetrySpanExt;
    use uuid::Uuid;

    /// Tracing information for error
    ///
    /// useful to log the tracing information to client
    /// without exposing the server's internal error
    pub struct Tracing {
        trace_id: TraceId,
        span_id: SpanId,
        log_id: Uuid,
    }

    impl Tracing {
        pub fn random() -> (Self, Uuid) {
            let log_id = Uuid::new_v4();
            (Self::new(log_id), log_id)
        }
        pub fn new(log_id: Uuid) -> Self {
            let span = tracing::error_span!("report");
            let ctx = span.context();
            let span_ref = ctx.span();
            let span_ctx = span_ref.span_context();
            let trace_id = span_ctx.trace_id();
            let span_id = span_ctx.span_id();

            Self {
                trace_id,
                span_id,
                log_id,
            }
        }
        pub fn report(self) -> String {
            format!(
                "trace_id: {}, span_id: {}, log_id: {}",
                self.trace_id, self.span_id, self.log_id
            )
        }
    }
}

#[cfg(not(feature = "insecure-print"))]
pub use insecure_print::Tracing;
