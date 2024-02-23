use super::Error;
use tonic::Code;

use crate::{
    assert_eq_error,
    client::connect,
    constants::SERVER,
    grpc::backend::{
        list_by_request, list_problem_request, problem_set_client::ProblemSetClient,
        ListProblemRequest, ProblemSortBy,
    },
};

// #[case::not_found(1, Code::NotFound)]
// #[case::large_number(1000, Code::InvalidArgument)]
pub async fn list(size: i64, code: Code) -> Result<(), Error> {
    let mut client = ProblemSetClient::with_origin(connect(), SERVER.try_into().unwrap());

    let res = client
        .list(ListProblemRequest {
            size,
            offset: Some(0),
            request: Some(list_problem_request::Request::Create(
                list_problem_request::Create {
                    sort_by: ProblemSortBy::Order as i32,
                    start_from_end: Some(false),
                },
            )),
        })
        .await;

    let err = res.unwrap_err();

    assert_eq_error!(err.code(), code, "list_problem should error");
    Ok(())
}

// #[case::not_found(1, Code::NotFound)]
// #[case::large_number(1000, Code::InvalidArgument)]
pub async fn list_by(size: i64, code: Code) -> Result<(), Error> {
    let mut client = ProblemSetClient::with_origin(connect(), SERVER.try_into().unwrap());

    let res = client
        .list_by_contest(crate::grpc::backend::ListByRequest {
            size,
            offset: Some(0),
            request: Some(list_by_request::Request::Create(list_by_request::Create {
                parent_id: 1,
                start_from_end: None,
            })),
            reverse: None,
        })
        .await;

    let err = res.unwrap_err();

    assert_eq_error!(err.code(), code, "list_problem should error");
    Ok(())
}
