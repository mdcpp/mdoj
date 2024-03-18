// use opentelemetry::trace::TraceContextExt;
// use tracing::Span;
// use tracing_opentelemetry::OpenTelemetrySpanExt;

/// Report error message to frontend
///
/// It may contain sensitive information, so it's guarded by feature flag
#[macro_export]
#[cfg(feature = "debug")]
macro_rules! report_internal {
    ($level:ident,$pattern:literal) => {{
        tracing::$level!($pattern);
        tonic::Status::internal($pattern.to_string())
    }};
    ($level:ident,$pattern:expr) => {{
        tracing::$level!("{}", $pattern);
        tonic::Status::internal($pattern.to_string())
    }};
    ($level:ident,$pattern:literal, $error:expr) => {{
        tracing::$level!($pattern, $error);
        tonic::Status::internal($error.to_string())
    }};
}

/// Log error message to infarstructure and report a error id to frontend
///
/// It doesn't contain sensitive information either on infrastructure or frontend
#[macro_export]
#[cfg(not(feature = "debug"))]
macro_rules! report_internal {
    ($level:ident,$pattern:literal) => {{
        let (msg, uuid) = crate::util::error::Tracing::random();
        tracing::$level!(uuid = uuid.to_string(), $pattern);
        tonic::Status::unknown(msg.to_string())
    }};
    ($level:ident,$pattern:expr) => {{
        let (msg, uuid) = crate::util::error::Tracing::random();
        tracing::$level!(uuid = uuid.to_string(), "{}", $pattern);
        tonic::Status::unknown(msg.to_string())
    }};
    ($level:ident,$pattern:literal, $error:expr) => {{
        let (msg, uuid) = crate::util::error::Tracing::random();
        tracing::$level!(uuid = uuid.to_string(), "{}", $pattern);
        tonic::Status::unknown(msg.to_string())
    }};
}

/// Check length of user inputted buffer
#[macro_export]
macro_rules! check_length {
    ($target:expr,$src:expr,$field:ident) => {
        paste::paste!{
            if $target<$src.$field.len(){
                return Err(Error::BufferTooLarge(stringify!($field)).into());
            }
        }
    };
    ($target:expr,$src:expr, $field:ident, $($ext:ident),+) => {
        check_length!($target,$src, $field);
        check_length!($target,$src, $($ext),+);
    };
}

/// Check length of user inputted buffer if it buffer is Some
///
/// It's useful because user may not input all fields
#[macro_export]
macro_rules! check_exist_length {
    ($target:expr,$src:expr,$field:ident) => {
        paste::paste!{
            if let Some(x)=$src.$field.as_ref(){
                if $target<x.len(){
                    return Err(Error::BufferTooLarge(stringify!($field)).into());
                }
            }
        }
    };
    ($target:expr,$src:expr, $field:ident, $($ext:ident),+) => {
        check_exist_length!($target,$src, $field);
        check_exist_length!($target,$src, $($ext),+);
    };
}

/// Fill many optional fields of active model at single line
///
/// This is useful when you want to update a model(user might not update all fields)
#[macro_export]
macro_rules! fill_exist_active_model {
    ($target:expr,$src:expr,$field:ident) => {
        if let Some(x) = $src.$field {
            $target.$field = ActiveValue::Set($crate::ofl!(x));
        }
    };
    ($target:expr,$src:expr, $field:ident, $($ext:ident),+) => {
        fill_exist_active_model!($target,$src, $field);
        fill_exist_active_model!($target,$src, $($ext),+);
    };
}

/// Fill many fields of active model at single line
#[macro_export]
macro_rules! fill_active_model {
    ($target:expr,$src:expr , $field:ident) => {
        $target.$field = ActiveValue::Set($crate::ofl!($src.$field));
    };
    ($target:expr,$src:expr, $field:ident, $($ext:ident),+) => {
        fill_active_model!($target,$src, $field);
        fill_active_model!($target,$src, $($ext),+);
    };
}

/// Overflow protection
///
/// check number and use `Residual`` operator to return [`Error::NumberTooLarge`]
#[macro_export]
macro_rules! ofl {
    ($n:expr) => {
        $n.try_into().map_err(|_| Error::NumberTooLarge)?
    };
}

/// Shorthanded `union`
///
/// This is an extremely dangerous macro to use
///
/// ```ignore
/// union!(
///     [Column::Id], // columns to select
///     user.find_related(Entity), // first query
///     Entity::find().filter(Column::Public.eq(true)) // second query
/// ).build_any(*builder)
/// ```
///
/// Note that it return [`sea_query::query::SelectStatement`],
/// which require you to build actual sql with builder backend
#[macro_export]
macro_rules! union {
    ($cols:expr,$a:expr,$b:expr) => {{
        use sea_orm::{QuerySelect, QueryTrait};
        $a.select_only()
            .columns($cols)
            .into_query()
            .union(
                sea_query::UnionType::Distinct,
                $b.select_only()
                    .columns($cols)
                    .select_only()
                    .columns($cols)
                    .into_query(),
            )
            .to_owned()
    }};
    ($cols:expr,$a:expr,$b:expr,$c:expr) => {{
        use sea_orm::{QuerySelect, QueryTrait};
        $a.select_only()
            .columns($cols)
            .into_query()
            .union(
                sea_query::UnionType::Distinct,
                $b.select_only()
                    .columns($cols)
                    .into_query()
                    .union(
                        sea_query::UnionType::Distinct,
                        $c.select_only().columns($cols).into_query(),
                    )
                    .to_owned(),
            )
            .to_owned()
    }};
}

/// Create `NonZeroU32` constant
#[macro_export]
macro_rules! NonZeroU32 {
    ($n:literal) => {
        match std::num::NonZeroU32::new($n) {
            Some(v) => v,
            None => panic!("expect non-zero u32"),
        }
    };
}

/// parse request
///
/// the req must have field `size`, `offset`, `request`
///
/// It does the following things:
///
/// 1. parse request into payload
/// 2. check if size is too large(>[`i32::MAX`])
/// 3. rate limiter with linear function
/// 4. extract sign from size
///
/// Be awared don't mess up the order of returned tuple
///
/// ```ignore
/// let (auth, rev, size, offset, pager) = parse_pager_param!(self, req);
/// ```
///
/// We use marco to parse request instead of a bunch of `From`
/// to decouple them from a macro
///
/// ```proto
/// message ListProblemRequest {
///   message Create {
///     required ProblemSortBy sort_by = 1;
///     optional bool start_from_end =2;
///     }
///   oneof request {
///     Create create = 1;
///     Paginator pager = 2;
///   }
///   required int64 size = 3;
///   optional uint64 offset = 4;
/// }
/// ````
#[macro_export]
macro_rules! parse_pager_param {
    ($self:expr,$req:expr) => {{
        let span = tracing::info_span!("parse").or_current();

        let (auth, req) = $self
            .parse_request_fn($req, |req| {
                ((req.size.saturating_abs() as u64) + req.offset() / 5 + 2)
                    .try_into()
                    .unwrap_or(u32::MAX)
            })
            .instrument(span)
            .await?;
        (
            auth,
            req.size < 0,
            req.size.saturating_abs() as u64,
            req.offset(),
            req.request.ok_or(Error::NotInPayload("request"))?,
        )
    }};
}
