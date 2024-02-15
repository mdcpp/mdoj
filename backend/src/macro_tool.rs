#[macro_export]
#[cfg(debug_assertions)]
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
#[cfg(not(debug_assertions))]
macro_rules! report_internal {
    ($level:ident,$pattern:literal) => {{
        tracing::$level!($pattern);
        tonic::Status::unknown("unknown error")
    }};
    ($level:ident,$pattern:expr) => {{
        tracing::$level!("{}", $pattern);
        tonic::Status::unknown("unknown error")
    }};
    ($level:ident,$pattern:literal, $error:expr) => {{
        tracing::$level!($pattern, $error);
        tonic::Status::unknown("unknown error")
    }};
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

/// bound check
#[macro_export]
macro_rules! bound {
    ($n:expr,$limit:literal) => {{
        if $n > $limit {
            return Err(Error::NumberTooLarge.into());
        }
        $n
    }};
}

#[macro_export]
macro_rules! partial_union {
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

#[macro_export]
macro_rules! NonZeroU32 {
    ($n:literal) => {
        match std::num::NonZeroU32::new($n) {
            Some(v) => v,
            None => panic!("expect non-zero u32"),
        }
    };
}
