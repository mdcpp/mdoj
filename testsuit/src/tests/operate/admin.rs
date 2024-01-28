use std::time::Duration;

use crate::{
    assert_eq_error,
    client::connect_with_token,
    grpc::backend::{
        submit_set_client::SubmitSetClient, CreateSubmitRequest, ProblemId, StateCode,
    },
    tests::{Error, State},
};
use async_std::task::sleep;

use uuid::Uuid;

use crate::constants::SERVER;

static CODE: &[u8] =
    b"a=io.read(\"*n\")\nb=io.read(\"*n\")\nc=io.read(\"*n\")\nio.write(tostring((a+b+c)))\n";

pub async fn submit(state: &mut State) -> Result<(), Error> {
    let mut client = SubmitSetClient::with_origin(
        connect_with_token(state.admin_token.as_ref().unwrap().signature.clone()),
        SERVER.try_into().unwrap(),
    );

    let res = client
        .create(CreateSubmitRequest {
            lang: "1c41598f-e253-4f81-9ef5-d50bf1e4e74f".to_owned(),
            problem_id: ProblemId {
                id: state.problem.as_mut().unwrap().0,
            },
            code: CODE.to_vec(),
            request_id: Uuid::new_v4().to_string(),
        })
        .await?
        .into_inner();

    // FIXME: follow was omit because we can't sleep in lua
    // There is no os binding in default lua plugin

    // let res=client.follow(res).await?.into_inner();

    sleep(Duration::from_millis(500)).await;

    let res = client.info(res).await?.into_inner();

    assert_eq_error!(res.state.code, StateCode::Ac as i32, "should AC");

    Ok(())
}
