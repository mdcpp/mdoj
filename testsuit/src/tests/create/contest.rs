use std::str::FromStr;

use crate::{
    assert_eq_error,
    client::connect_with_token,
    grpc::backend::{
        contest_set_client::ContestSetClient, create_contest_request, create_testcase_request,
        testcase_set_client::TestcaseSetClient, CreateContestRequest, CreateTestcaseRequest,
    },
    tests::{Error, State},
};
use prost_types::Timestamp;
use uuid::Uuid;

use crate::constants::SERVER;

use super::StartOfId;

pub async fn create(state: &mut State) -> Result<(), Error> {
    let mut client = ContestSetClient::with_origin(
        connect_with_token(state.admin_token.as_ref().unwrap().signature.clone()),
        SERVER.try_into().unwrap(),
    );

    let res = client
        .create(CreateContestRequest {
            info: create_contest_request::Info {
                title: "testing contest".to_string(),
                begin: Timestamp::from_str("1970-01-01T00:00:00Z").unwrap(),
                end: Timestamp::from_str("2050-01-01T00:00:00Z").unwrap(),
                tags: "testsuit search_filter_1 search_filter_2".to_owned(),
                content: "THIS IS A TESTING CONTEST, seeing this in your production deployment is dangerous.\nshould not be search".to_owned(),
                password: Some("password".to_owned()),
            },
            request_id: Uuid::new_v4().to_string(),
        })
        .await
        .unwrap();

    let res = res.into_inner();
    state.contest = Some(StartOfId(res.id));

    Ok(())
}
