#[macro_export]
#[cfg(feature = "unsecured-log")]
macro_rules! report_internal {
    ($level:ident,$pattern:literal) => {{
        log::$level!($pattern);
        tonic::Status::internal($error.to_string())
    }};
    ($level:ident,$pattern:literal, $error:expr) => {{
        log::$level!($pattern, $error);
        tonic::Status::internal($error.to_string())
    }};
}

#[macro_export]
macro_rules! report_internal {
    ($level:ident,$pattern:literal) => {{
        log::$level!($pattern);
        tonic::Status::unknown("unknown error")
    }};
    ($level:ident,$pattern:literal, $error:expr) => {{
        log::$level!($pattern, $error);
        tonic::Status::unknown("unknown error")
    }};
}

#[macro_export]
macro_rules! fill_exist_active_model {
    ($target:expr,$src:expr , $field:ident) => {
        if let Some(x) = $src.$field {
            $target.$field = ActiveValue::Set(crate::ofl!(x));
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
        $target.$field = ActiveValue::Set(crate::ofl!($src.$field));
    };
    ($target:expr,$src:expr, $field:ident, $($ext:ident),+) => {
        fill_active_model!($target,$src, $field);
        fill_active_model!($target,$src, $($ext),+);
    };
}

#[macro_export]
macro_rules! ofl {
    ($n:expr) => {
        $n.try_into().map_err(|_| Error::NumberTooLarge)?
    };
}
