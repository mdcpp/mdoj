use async_std::task;
use rstest::*;
use tonic::Code;

use crate::{
    client::connect,
    constant::SERVER,
    grpc::backend::{
        list_by_request, list_problem_request, problem_set_client::ProblemSetClient,
        ListProblemRequest, ProblemSortBy,
    },
};

#[rstest]
#[case::not_found(1, Code::NotFound)]
#[case::large_number(1000, Code::InvalidArgument)]
async fn list_problem(#[case] size: u64, #[case] code: Code) {
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

    assert_eq!(err.code(), code)
}

#[rstest]
#[case::not_found(1, Code::NotFound)]
#[case::large_number(1000, Code::InvalidArgument)]
async fn list_problem_by_contest(#[case] size: u64, #[case] code: Code) {
    let mut client = ProblemSetClient::with_origin(connect(), SERVER.try_into().unwrap());

    let res = client
        .list_by_contest(crate::grpc::backend::ListByRequest {
            size,
            offset: Some(0),
            request: Some(list_by_request::Request::Create(list_by_request::Create {
                parent_id: 1,
                start_from_end: false,
            })),
            reverse: None,
        })
        .await;

    let err = res.unwrap_err();

    assert_eq!(err.code(), code)
}
