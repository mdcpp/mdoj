use crate::{
    client::connect_with_token,
    constants::SERVER,
    grpc::backend::{
        testcase_set_client::TestcaseSetClient, AddTestcaseToProblemRequest, ProblemId, TestcaseId,
    },
    tests::{Error, State},
};

pub async fn testcase(state: &mut State) -> Result<(), Error> {
    let mut client = TestcaseSetClient::with_origin(
        connect_with_token(state.admin_token.as_ref().unwrap().signature.clone()),
        SERVER.try_into().unwrap(),
    );

    for i in 0..3 {
        client
            .add_to_problem(AddTestcaseToProblemRequest {
                testcase_id: TestcaseId {
                    id: state.testcase.as_ref().unwrap().0 + i,
                },
                problem_id: ProblemId {
                    id: state.problem.as_ref().unwrap().0,
                },
            })
            .await?;
    }

    Ok(())
}
