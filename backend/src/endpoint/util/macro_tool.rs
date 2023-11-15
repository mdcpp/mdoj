#[macro_export]
macro_rules! impl_endpoint {
    ($name:ident) => {
        impl_id!($name);
        impl_intel!($name);
        impl_create_request!($name);
        impl_update_request!($name);
    };
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
            impl Transform<[<$name Id>]> for i32 {
                fn into(self) -> [<$name Id>] {
                    [<$name Id>] { id: self }
                }
            }
        }
    };
}

#[macro_export]
macro_rules! impl_create_request {
    ($name:ident) => {
        paste::paste! {
            impl TryTransform<[<create_ $name:lower _request>]::Info, Error> for [<Create $name:camel Request>]{
                fn try_into(self) -> Result<[<create_ $name:lower _request>]::Info, Error> {
                    let info = self.info.ok_or(Error::NotInPayload("info"))?;
                    Ok(info)
                }
            }
        }
    };
}

#[macro_export]
macro_rules! impl_update_request {
    ($name:ident) => {
        paste::paste! {
            impl TryTransform<([<update_ $name:lower _request>]::Info,i32), Error> for [<Update $name:camel Request>]{
                fn try_into(self) -> Result<([<update_ $name:lower _request>]::Info,i32), Error> {
                    let info = self.info.ok_or(Error::NotInPayload("info"))?;
                    let id = self.id.map(|x| x.id).ok_or(Error::NotInPayload("id"))?;
                    Ok((info, id))
                }
            }
        }
    };
}

#[macro_export]
macro_rules! impl_intel {
    ($name:ident) => {
        paste::paste! {
            pub struct [<$name:camel Intel>];
            impl IntelTrait for [<$name:camel Intel>] {
                const NAME: &'static str = " $name ";
                type Entity = Entity;
                type PartialModel = [<Partial $name:camel>];
                type InfoArray = [<$name:camel s>];
                type FullInfo = [<$name:camel FullInfo>];
                type Info =  [<$name:camel Info>];
                type PrimaryKey = i32;
                type Id = [<$name:camel Id>];
                type UpdateInfo = [<update_ $name:lower _request>]::Info;
                type CreateInfo = [<create_ $name:lower _request>]::Info;
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
