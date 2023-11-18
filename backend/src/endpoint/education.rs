use super::endpoints::*;
use super::tools::*;

use crate::grpc::backend::education_set_server::*;
use crate::grpc::backend::*;

use entity::{education::*, *};
#[async_trait]
impl EducationSet for Arc<Server> {
    async fn list(
        &self,
        req: Request<ListRequest>,
    ) -> Result<Response<ListEducationResponse>, Status> {
        Err(Status::unimplemented("unimplemented"))
    }
    async fn search_by_text(
        &self,
        req: Request<TextSearchRequest>,
    ) -> Result<Response<ListEducationResponse>, Status> {
        Err(Status::unimplemented("unimplemented"))
    }
    async fn search_by_tag(
        &self,
        req: Request<TextSearchRequest>,
    ) -> Result<Response<ListEducationResponse>, Status> {
        Err(Status::unimplemented("unimplemented"))
    }
    async fn full_info(
        &self,
        req: Request<EducationId>,
    ) -> Result<Response<EducationFullInfo>, Status> {
        Err(Status::unimplemented("unimplemented"))
    }
    async fn create(&self, req: Request<CreateEducationRequest>) -> Result<Response<()>, Status> {
        Err(Status::unimplemented("unimplemented"))
    }
    async fn update(&self, req: Request<UpdateEducationRequest>) -> Result<Response<()>, Status> {
        Err(Status::unimplemented("unimplemented"))
    }
    async fn remove(&self, req: Request<EducationId>) -> Result<Response<()>, Status> {
        Err(Status::unimplemented("unimplemented"))
    }
    async fn link(&self, req: Request<EducationLink>) -> Result<Response<()>, Status> {
        Err(Status::unimplemented("unimplemented"))
    }
    async fn unlink(&self, req: Request<EducationLink>) -> Result<Response<()>, Status> {
        Err(Status::unimplemented("unimplemented"))
    }
    async fn full_info_by_problem(
        &self,
        req: Request<EducationLink>,
    ) -> Result<Response<EducationFullInfo>, Status> {
        Err(Status::unimplemented("unimplemented"))
    }
}
