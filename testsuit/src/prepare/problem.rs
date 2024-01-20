// use std::fmt::format;

// use crate::{
//     client::connect_with_token,
//     empty::login::admin_token,
//     grpc::backend::{create_problem_request, MatchRule},
// };
// use async_std::task;
// use rstest::*;
// use tonic::{metadata::MetadataValue, transport::Channel, Code, Request};
// use uuid::Uuid;

// use crate::{
//     client::connect,
//     constant::SERVER,
//     grpc::backend::{problem_set_client::ProblemSetClient, CreateProblemRequest},
// };

// #[rstest]
// async fn create_problem(#[future] admin_token: String) {
//     let mut client = ProblemSetClient::with_origin(
//         connect_with_token(admin_token.await),
//         SERVER.try_into().unwrap(),
//     );

//     let mut last = None;
//     for secquence in 1..11 {
//         let res = client
//             .create(CreateProblemRequest {
//                 info: create_problem_request::Info {
//                     title: format!("Problem {}", secquence),
//                     difficulty: secquence,
//                     time: 1000 * 1000,
//                     memory: 1024 * 1024 * 128,
//                     tags: "problem test".to_owned(),
//                     content: format!("description for problem {}", secquence),
//                     match_rule: MatchRule::ExactSame as i32,
//                     order: 0.01 * ((1 + secquence) as f32),
//                 },
//                 request_id: Uuid::new_v4().to_string(),
//             })
//             .await
//             .unwrap();

//         let res = res.into_inner();
//         if let Some(x) = last {
//             assert_eq!(x + 1, res.id);
//         }
//         last = Some(res.id);
//     }
// }
