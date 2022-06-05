use openssl::symm::Mode;
use openssl::{aes,base64};
use serde::{Deserialize, Serialize};
use rand::prelude::*;

const AES_KEY: &[u8; 32] = include_bytes!["../../config/aes"];

#[derive(Serialize, Deserialize)]
pub struct TokenPayload<'a> {
    pub username: &'a str,
    pub password: &'a str,
}

pub struct AuthPayload<'a> {
    pub username: &'a str,
    pub password: &'a str,
}

pub fn generate<'a>(payload: TokenPayload<'a>,salt:&[u8; 32])->String {
    todo!()
}

pub async fn revoke() {

}
pub async fn verify() {}

