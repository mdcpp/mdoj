#[macro_export]
macro_rules! impl_id {
    ($name:ident) => {
        paste::paste! {
            impl Transform<i32> for [<$name Id>] {
                fn into(self) -> i32 {
                    self.id
                }
            }
            impl Transform<[<$name Id>]> for i32 {
                fn into(self) -> [<$name Id>] {
                    [<$name Id>] { id: self }
                }
            }
        }
    };
}

#[macro_export]
macro_rules! fill_exist_active_model {
    ($target:expr,$src:expr , $field:ident) => {
        if let Some(x) = $src.$field {
            $target.$field = ActiveValue::Set(x);
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
        $target.$field = ActiveValue::Set($src.$field);
    };
    ($target:expr,$src:expr, $field:ident, $($ext:ident),+) => {
        fill_active_model!($target,$src, $field);
        fill_active_model!($target,$src, $($ext),+);
    };
}
