use crate::entity;
use openssl::aes;
use sea_orm::prelude::*;

const AES_KEY: &[u8; 32] = include_bytes!["../../config/aes"];

pub struct AuthPayload<'a> {
    username: &'a str,
    password: &'a str,
}

pub async fn generate<'a>(payload: AuthPayload<'a>) {
    entity::user_table::Entity::find()
        .filter(entity::user_table::Column::NameUser.eq(payload.username))
        .filter(entity::user_table::Column::HashedPassword.eq(payload.password));
}
pub async fn revoke() {}
pub async fn verify() {}
