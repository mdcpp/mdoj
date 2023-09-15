// use crate::grpc::proto::*;

use tonic::{async_trait, Request, Response};

use crate::{
    grpc::proto::prelude::problem_set_server::ProblemSet, grpc::proto::prelude::*, Server,
};

#[async_trait]
impl ProblemSet for Server {
    async fn list(
        &self,
        request: Request<ListRequest>,
    ) -> Result<Response<Problems>, tonic::Status> {
        todo!()
    }

    async fn search_by_text(
        &self,
        request: Request<SearchByTextRequest>,
    ) -> Result<Response<Problems>, tonic::Status> {
        todo!()
    }

    async fn search_by_tag(
        &self,
        request: Request<SearchByTagRequest>,
    ) -> Result<Response<Problems>, tonic::Status> {
        todo!()
    }

    async fn full_info(
        &self,
        request: Request<ProblemId>,
    ) -> Result<Response<ProblemFullInfo>, tonic::Status> {
        todo!()
    }

    async fn create(
        &self,
        request: Request<ProblemFullInfo>,
    ) -> Result<Response<()>, tonic::Status> {
        todo!()
    }

    async fn update(
        &self,
        request: Request<ProblemFullInfo>,
    ) -> Result<Response<()>, tonic::Status> {
        todo!()
    }

    async fn remove(
        &self,
        request: Request<ProblemId>,
    ) -> Result<Response<()>, tonic::Status> {
        todo!()
    }

    async fn link(
        &self,
        request: Request<ProblemLink>,
    ) -> Result<Response<()>, tonic::Status> {
        todo!()
    }

    async fn unlink(
        &self,
        request: Request<ProblemLink>,
    ) -> Result<Response<()>, tonic::Status> {
        todo!()
    }

    async fn add_test(
        &self,
        request: Request<Testcase>,
    ) -> Result<Response<()>, tonic::Status> {
        todo!()
    }

    async fn remove_test(
        &self,
        request: Request<TestcaseId>,
    ) -> Result<Response<()>, tonic::Status> {
        todo!()
    }
}
