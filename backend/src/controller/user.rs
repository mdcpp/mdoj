use super::token;
use super::crypto;
use crate::entity;
use openssl::sha::sha256;
use sea_orm::prelude::*;
use crate::entity::user_table;
use crate::entity::prelude::UserTable;

pub struct CtrlUser {
    pub model: Option<entity::user_table::Model>,
}

impl CtrlUser {
    async fn verify_identity<'a>(&mut self, payload: token::AuthPayload<'a>,conn:DatabaseConnection) -> bool {
        let bytea=sha256(payload.password.as_bytes());
        let user:Option<user_table::Model>=UserTable::find()
            .filter(user_table::Column::NameUser.eq(payload.username))
            .filter(user_table::Column::HashedPassword.eq(bytea.as_slice())).one(&conn).await.unwrap();
        match user{
            Some(user) => {
                self.model=Some(user);
                true
            },
            None => false,
        }
    }
    async fn generate_token(){
        todo!();
    }
    async fn verify_token(){
        todo!();
    }
}


#[cfg(test)]
mod test{
    use super::*;

}