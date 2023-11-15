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

// macro_rules! make_public{
//     (
//      $(#[$meta:meta])*
//      $vis:vis struct $struct_name:ident {
//         $(
//         $(#[$field_meta:meta])*
//         $field_vis:vis $field_name:ident : $field_type:ty
//         ),*$(,)+
//     }
//     ) => {

//             $(#[$meta])*
//             pub struct $struct_name{
//                 $(
//                 $(#[$field_meta:meta])*
//                 pub $field_name : $field_type,
//                 )*
//             }
//     }
// }

// #[macro_export]
// macro_rules! parse_option {
//     ($payload:expr,$field:ident) => {
//         paste::paste! {
//             $payload
//             .$field
//             .ok_or(tonic::Status::invalid_argument(format!("{} is required",stringify!($ident))))?
//         }
//     };
// }

// macro_rules!  {
//     () => {

//     };
// }

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
