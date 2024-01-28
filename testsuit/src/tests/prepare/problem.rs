use crate::{
    assert_eq_error,
    client::connect_with_token,
    grpc::backend::{create_problem_request, MatchRule},
    tests::{Error, State},
};
use uuid::Uuid;

use crate::{
    constants::SERVER,
    grpc::backend::{problem_set_client::ProblemSetClient, CreateProblemRequest},
};

use super::StartOfId;

pub async fn create(state: &mut State) -> Result<(), Error> {
    let mut client = ProblemSetClient::with_origin(
        connect_with_token(state.admin_token.as_ref().unwrap().signature.clone()),
        SERVER.try_into().unwrap(),
    );

    let mut last = None;
    for secquence in 1..11 {
        let res = client
            .create(CreateProblemRequest {
                info: create_problem_request::Info {
                    title: format!("Problem {}", secquence),
                    difficulty: secquence,
                    time: 1000 * 1000,
                    memory: 1024 * 1024 * 128,
                    tags: "problem test".to_owned(),
                    content: format!("description for problem {}\nInputs: x,y,z separated by space\nOutput: x+y+z", secquence),
                    match_rule: MatchRule::IgnoreSnl as i32,
                    order: 0.01 * ((1 + secquence) as f32),
                },
                request_id: Uuid::new_v4().to_string(),
            })
.await?;

        let res = res.into_inner();
        if let Some(x) = last {
            assert_eq_error!(x + 1, res.id, "id generator should be sequential");
        } else {
            state.problem = Some(StartOfId(res.id));
        }
        last = Some(res.id);
    }

    Ok(())
}
