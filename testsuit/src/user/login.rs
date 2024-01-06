// use crate::{
//     case::Case,
//     client::connection,
//     constant,
//     grpc::backend::{token_set_client::TokenSetClient, LoginRequest},
// };

// use super::State;

// pub struct AdminLogin;

// #[tonic::async_trait]
// impl Case<State> for AdminLogin {
//     const NAME: &'static str = "login as admin@admin";

//     async fn run(&self, state: &mut State) -> Result<(), String> {
//         let mut client =
//             TokenSetClient::with_origin(connection(), constant::SERVER.try_into().unwrap());

//         let res = client
//             .create(LoginRequest {
//                 username: constant::ADMIN.to_string(),
//                 password: constant::ADMIN_PWD.to_string(),
//                 expiry: None,
//             })
//             .await
//             .unwrap();

//         let res = res.into_inner();

//         assert!(res.permission.can_root);
//         assert!(!res.permission.can_link);
//         assert!(!res.permission.can_manage_announcement);
//         assert!(!res.permission.can_manage_chat);
//         assert!(!res.permission.can_manage_contest);
//         assert!(!res.permission.can_manage_education);
//         assert!(!res.permission.can_manage_problem);
//         assert!(!res.permission.can_manage_submit);
//         assert!(!res.permission.can_manage_user);
//         assert!(!res.permission.can_publish);
//         assert!(!res.permission.can_imgur);

//         state.token = Some(res.token.signature);
//         Ok(())
//     }
// }
