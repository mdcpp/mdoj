use super::endpoints::*;
use super::tools::*;

use crate::grpc::prelude::testcase_set_server::TestcaseSet;
use crate::{endpoint::*, grpc::prelude::*, impl_id, Server};

use entity::{testcase::*, *};
use tonic::*;

type TonicStream<T> = std::pin::Pin<Box<dyn tokio_stream::Stream<Item = Result<T, Status>> + Send>>;

pub struct TestcaseIntel;

impl IntelTrait for TestcaseIntel {
    const NAME: &'static str = "testcase";

    type Entity = Entity;

    type PartialModel = PartialTestcase;

    type PrimaryKey = i32;

    type Id = TestcaseId;

    type InfoArray = Testcases;

    type FullInfo = TestcaseFullInfo;

    type Info = TestcaseInfo;

    type UpdateInfo = update_testcase_request::Info;

    type CreateInfo = create_testcase_request::Info;
}

impl_id!(Testcase);

#[async_trait]
impl Intel<TestcaseIntel> for Server {
    fn ro_filter<S>(query: S, auth: Auth) -> Result<S, Error>
    where
        S: QueryFilter,
    {
        todo!()
    }

    fn rw_filter<S>(query: S, auth: Auth) -> Result<S, Error>
    where
        S: QueryFilter,
    {
        todo!()
    }

    fn can_create(auth: Auth) -> Result<i32, Error> {
        todo!()
    }

    async fn update_model(model: Model, info: update_testcase_request::Info) -> Result<i32, Error> {
        todo!()
    }

    async fn create_model(
        model: create_testcase_request::Info,
        user_id: i32,
    ) -> Result<i32, Error> {
        todo!()
    }
}

impl TryTransform<create_testcase_request::Info, Error> for CreateTestcaseRequest {
    fn try_into(self) -> std::result::Result<create_testcase_request::Info, Error> {
        todo!()
    }
}

impl TryTransform<update_testcase_request::Info, Error> for UpdateTestcaseRequest {
    fn try_into(self) -> std::result::Result<update_testcase_request::Info, Error> {
        todo!()
    }
}

impl BaseEndpoint<TestcaseIntel> for Server {}

impl TryTransform<(update_testcase_request::Info, i32), Error> for UpdateTestcaseRequest {
    fn try_into(self) -> std::result::Result<(update_testcase_request::Info, i32), Error> {
        todo!()
    }
}

#[async_trait]
impl TestcaseSet for Server {
    async fn create(
        &self,
        request: Request<CreateTestcaseRequest>,
    ) -> Result<Response<TestcaseId>, Status> {
        BaseEndpoint::<TestcaseIntel>::create(self, request)
            .await
            .map_err(|x| x.into())
    }

    async fn update(
        &self,
        request: Request<UpdateTestcaseRequest>,
    ) -> Result<Response<()>, Status> {
        BaseEndpoint::<TestcaseIntel>::update(self, request)
            .await
            .map_err(|x| x.into())
    }

    async fn remove(&self, request: Request<TestcaseId>) -> Result<Response<()>, Status> {
        BaseEndpoint::<TestcaseIntel>::remove(self, request)
            .await
            .map_err(|x| x.into())
    }

    async fn link(&self, request: Request<TestcaseLink>) -> Result<Response<()>, Status> {
        todo!()
    }

    async fn unlink(&self, request: Request<TestcaseLink>) -> Result<Response<()>, Status> {
        todo!()
    }

    async fn full_info_by_problem(
        &self,
        request: Request<TestcaseLink>,
    ) -> Result<Response<TestcaseFullInfo>, Status> {
        todo!()
    }

    #[doc = " Server streaming response type for the ListByProblem method."]
    type ListByProblemStream = TonicStream<TestcaseInfo>;

    async fn list_by_problem(
        &self,
        request: Request<TestcaseLink>,
    ) -> Result<Response<Self::ListByProblemStream>, Status> {
        todo!()
    }
}
