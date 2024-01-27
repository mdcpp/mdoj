use crate::{
    assert_eq_error,
    client::connect_with_token,
    grpc::backend::{create_user_request, user_set_client::UserSetClient, CreateUserRequest, Role},
    tests::{Error, State},
};
use uuid::Uuid;

use crate::constants::SERVER;

use super::StartOfId;

pub async fn create(state: &mut State) -> Result<(), Error> {
    let mut client = UserSetClient::with_origin(
        connect_with_token(state.admin_token.as_ref().unwrap().signature.clone()),
        SERVER.try_into().unwrap(),
    );

    let mut last = None;
    for secquence in 1..3 {
        let res = client
            .create(CreateUserRequest {
                info: create_user_request::Info {
                    username: format!("user{}", secquence),
                    password: secquence.to_string(),
                    role: Role::User as i32,
                },
                request_id: Uuid::new_v4().to_string(),
            })
            .await?;

        let res = res.into_inner();
        if let Some(x) = last {
            assert_eq_error!(x + 1, res.id, "id generator should be sequential");
        } else {
            state.user = Some(StartOfId(res.id));
        }
        last = Some(res.id);
    }

    Ok(())
}
