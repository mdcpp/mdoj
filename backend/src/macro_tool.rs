// use opentelemetry::trace::TraceContextExt;
// use tracing::Span;
// use tracing_opentelemetry::OpenTelemetrySpanExt;

/// Report error message to frontend
///
/// It may contain sensitive information, so it's guarded by feature flag
#[macro_export]
#[cfg(feature = "insecure-print")]
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

/// Log error message to infrastructure and report an error id to frontend
///
/// It doesn't contain sensitive information either on infrastructure or frontend
#[macro_export]
#[cfg(not(feature = "insecure-print"))]
macro_rules! report_internal {
    ($level:ident,$pattern:literal) => {{
        let (msg, uuid) = crate::util::error::Tracing::random();
        tracing::$level!(uuid = uuid.to_string(), $pattern);
        tonic::Status::unknown(msg.report())
    }};
    ($level:ident,$error:expr) => {{
        let (msg, uuid) = crate::util::error::Tracing::random();
        tracing::$level!(uuid = uuid.to_string(), "{}", $error);
        tonic::Status::unknown(msg.report())
    }};
    ($level:ident,$pattern:literal, $error:expr) => {{
        let (msg, uuid) = crate::util::error::Tracing::random();
        tracing::$level!(uuid = uuid.to_string(), $pattern, $error);
        tonic::Status::unknown(msg.report())
    }};
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
