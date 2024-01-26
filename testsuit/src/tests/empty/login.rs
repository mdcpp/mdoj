use super::Error;
use crate::{
    assert_eq_error,
    client::connect,
    constant::*,
    grpc::backend::{token_set_client::TokenSetClient, LoginRequest, Role},
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct AdminToken {
    signature: String,
}

pub async fn login() -> Result<AdminToken, super::Error> {
    let mut client = TokenSetClient::with_origin(connect(), SERVER.try_into().unwrap());

    let res = client
        .create(LoginRequest {
            username: ADMIN.to_owned(),
            password: ADMIN_PWD.to_owned(),
            expiry: None,
        })
        .await
        .unwrap();

    let res = res.into_inner();

    assert_eq_error!(res.role(), Role::Root, "admin@admin login fail");

    Ok(AdminToken {
        signature: res.token.signature,
    })
}
