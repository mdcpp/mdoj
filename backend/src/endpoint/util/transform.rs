use tonic::async_trait;

pub trait TryTransform<I, E> {
    fn try_into(self) -> Result<I, E>;
}

/// not only into
pub trait Transform<I> {
    fn into(self) -> I;
}

#[macro_export]
macro_rules! impl_id {
    ($name:ident) => {
        paste::paste! {
            impl Transform<i32> for [<$name Id>] {
                fn into(self) -> i32 {
                    self.id
                }
            }
        }
    };
}

#[async_trait]
pub trait AsyncTransform<T> {
    async fn into(self) -> T;
}

#[macro_export]
macro_rules! parse_option {
    ($payload:expr,$field:ident) => {
        paste::paste! {
            $payload
            .$field
            .ok_or(tonic::Status::invalid_argument(format!("{} is required",stringify!($ident))))?
        }
    };
}

// macro_rules! match_col {
//     ($target:expr, $field:ident) => {
//         SortBy::$field => Column::$field
//     };
//     ($target:expr, $field:ident, $($ext:ident),+) => {
//         ,match_col($target, $field),
//         match_col($target, $($ext),+)
//     };
// }

// #[macro_export]
// macro_rules! match_sort {
//     ($target:expr, $field:ident, $($ext:ident),+) => {
//         paste::paste!{
//             impl Transform<<Entity as EntityTrait>::Column> for SortBy {
//                 fn into(self) -> <<Entity as EntityTrait>::Column {
//                     match self {
//                         match_col!(self, $field, $($ext),+),
//                         _ => Column::Id,
//                     }
//                 }
//             }
//         }
//     };
// }
