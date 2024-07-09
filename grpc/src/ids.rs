use crate::backend::Id;

impl From<i32> for Id {
    fn from(value: i32) -> Self {
        Id { id: value }
    }
}

impl From<Id> for i32 {
    fn from(value: Id) -> Self {
        value.id
    }
}
