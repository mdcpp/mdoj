use async_std::task;
use cached::proc_macro::cached;
use rstest::*;

use crate::{
    client::connect,
    constant::*,
    grpc::backend::{token_set_client::TokenSetClient, LoginRequest, Role},
};

#[fixture]
pub async fn admin_token() -> String {
    inner_admin_token().await
}

#[cached]
pub async fn inner_admin_token() -> String {
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

    assert_eq!(res.role(), Role::Root);

    res.token.signature
}

#[rstest]
async fn test(#[future] admin_token: String) {
    admin_token.await;
}
