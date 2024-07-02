use crate::backend::*;
use paste::paste;

macro_rules! impl_ids {
    ($entity:ident) => {
        paste! {
            impl From<i32> for [<$entity Id>] {
                fn from(value: i32) -> Self {
                    Self { id: value }
                }
            }

            impl From<[<$entity Id>]> for i32 {
                fn from(value: [<$entity Id>]) -> Self {
                    value.id
                }
            }
        }
    };
}

impl_ids!(Problem);
impl_ids!(Announcement);
impl_ids!(Contest);
impl_ids!(Chat);
impl_ids!(Testcase);
impl_ids!(Education);
impl_ids!(Submit);
impl_ids!(User);
