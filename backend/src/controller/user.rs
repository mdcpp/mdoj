use super::token;
use crate::entity;

pub struct CtrlUser {
    pub model: Option<entity::user_table::Model>,
}

impl CtrlUser {
    async fn verify_identity<'a>(&mut self, payload: token::AuthPayload<'a>) -> bool {
        // entity::prelude::UserTable.find(predicate);
        todo!();
    }
}
