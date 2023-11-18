use super::endpoints::*;
use super::tools::*;

use crate::grpc::backend::user_set_server::*;
use crate::grpc::backend::*;

use entity::{user::*, *};

impl From<i32> for UserId {
    fn from(value: i32) -> Self {
        Self { id: value }
    }
}

impl From<UserId> for i32 {
    fn from(value: UserId) -> Self {
        value.id
    }
}

#[async_trait]
impl UserSet for Server {
    async fn list(&self, req: Request<ListRequest>) -> Result<Response<ListUserResponse>, Status> {
        Err(Status::unimplemented("unimplemented"))
    }
    async fn search_by_text(
        &self,
        req: Request<TextSearchRequest>,
    ) -> Result<Response<ListUserResponse>, Status> {
        Err(Status::unimplemented("unimplemented"))
    }
    async fn full_info(&self, req: Request<UserId>) -> Result<Response<UserFullInfo>, Status> {
        Err(Status::unimplemented("unimplemented"))
    }
    async fn create(&self, req: Request<CreateUserRequest>) -> Result<Response<()>, Status> {
        Err(Status::unimplemented("unimplemented"))
    }
    async fn update(&self, req: Request<UpdateUserRequest>) -> Result<Response<()>, Status> {
        Err(Status::unimplemented("unimplemented"))
    }
    async fn remove(&self, req: Request<UserId>) -> Result<Response<()>, Status> {
        Err(Status::unimplemented("unimplemented"))
    }
}
