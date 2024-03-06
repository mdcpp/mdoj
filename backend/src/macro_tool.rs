use opentelemetry::trace::TraceContextExt;
use tracing::Span;
use tracing_opentelemetry::OpenTelemetrySpanExt;

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

#[macro_export]
#[cfg(not(feature = "debug"))]
macro_rules! report_internal {
    ($level:ident,$pattern:literal) => {{
        tracing::$level!($pattern);
        tonic::Status::unknown(crate::macro_tool::debug_msg())
    }};
    ($level:ident,$pattern:expr) => {{
        tracing::$level!("{}", $pattern);
        tonic::Status::unknown(crate::macro_tool::debug_msg())
    }};
    ($level:ident,$pattern:literal, $error:expr) => {{
        tracing::$level!($pattern, $error);
        tonic::Status::unknown(crate::macro_tool::debug_msg())
    }};
}

pub fn debug_msg() -> String {
    let ctx = Span::current().context();
    let ctx_span = ctx.span();
    let span_ctx = ctx_span.span_context();
    let trace_id = span_ctx.trace_id();
    let span_id = span_ctx.span_id();

    format!("trace_id: {}, span_id: {}", trace_id, span_id)
}

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

/// overflow protection
#[macro_export]
macro_rules! ofl {
    ($n:expr) => {
        $n.try_into().map_err(|_| Error::NumberTooLarge)?
    };
}

/// shorthanded union macro
///
/// Note that it return select query column(for partial model)
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
