use crate::{
    assert_eq_error,
    client::connect_with_token,
    grpc::backend::{
        create_testcase_request, testcase_set_client::TestcaseSetClient, CreateTestcaseRequest,
    },
    tests::{Error, State},
};
use uuid::Uuid;

use crate::constants::SERVER;

use super::StartOfId;

pub async fn create(state: &mut State) -> Result<(), Error> {
    let mut client = TestcaseSetClient::with_origin(
        connect_with_token(state.admin_token.as_ref().unwrap().signature.clone()),
        SERVER.try_into().unwrap(),
    );

    let mut last = None;
    for secquence in 1..11 {
        let res = client
            .create(CreateTestcaseRequest {
                info: create_testcase_request::Info {
                    score: secquence,
                    input: b"2 3 4".to_vec(),
                    output: b"10".to_vec(),
                },
                request_id: Uuid::new_v4().to_string(),
            })
            .await
            .unwrap();

        let res = res.into_inner();
        if let Some(x) = last {
            assert_eq_error!(x + 1, res.id, "id generator should be sequential");
        } else {
            state.testcase = Some(StartOfId(res.id));
        }
        last = Some(res.id);
    }

    Ok(())
}
