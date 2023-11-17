use super::endpoints::*;
use super::tools::*;
use super::util::filter::Filter;
use super::util::pagination::*;

use crate::grpc::backend::problem_set_server::*;
use crate::server::Server;
use crate::{endpoint::*, grpc::backend::*, impl_id};

use entity::{problem::*, *};
use sea_orm::QueryOrder;
use sea_orm::Select;
use tonic::*;

#[tonic::async_trait]
impl Filter for Entity{
    async fn read_filter(query:Select<Self>,auth: &Auth) -> Result<Select<Self>,Error> {
        todo!()
    }

    async fn write_filter(query:Select<Self>,auth: &Auth) -> Result<Select<Self>,Error> {
        todo!()
    }
}

#[tonic::async_trait]
impl PagerTrait for Entity {
    const TYPE_NUMBER: i32 = 11223;

    const COL_ID: Column = Column::Id;

    const COL_TEXT: &'static [Column] = &[Column::Title, Column::Tags];

    type ParentMarker = HasParent<contest::Entity>;

    fn sort(select: Select<Self>, sort: SortBy, reverse: bool) -> Select<Self> {
        match sort {
            SortBy::UploadDate => select.order_by_desc(Column::CreateAt),
            SortBy::AcRate => select.order_by_desc(Column::AcRate),
            SortBy::SubmitCount => select.order_by_desc(Column::SubmitCount),
            SortBy::Difficulty => select.order_by_asc(Column::Difficulty),
            _ => select,
        }
    }

    fn get_id(model: &Self::Model) -> i32 {
        model.id
    }

    async fn query_filter(select: Select<Self>, auth: &Auth) -> Result<Select<Self>, Error> {
        Entity::read_filter(select, auth).await
    }
}

#[tonic::async_trait]
impl ProblemSet for Server {
    async fn list(
        &self,
        req: Request<ListRequest>,
    ) -> Result<Response<ListProblemResponse>, Status> {
        todo!()
    }

    async fn search_by_text(
        &self,
        req: Request<TextSearchRequest>,
    ) -> Result<Response<ListProblemResponse>, Status> {
        todo!()
    }

    async fn search_by_tag(
        &self,
        req: Request<TextSearchRequest>,
    ) -> Result<Response<ListProblemResponse>, Status> {
        todo!()
    }

    async fn full_info(
        &self,
        req: Request<ProblemId>,
    ) -> Result<Response<ProblemFullInfo>, Status> {
        todo!()
    }

    async fn create(
        &self,
        req: Request<CreateProblemRequest>,
    ) -> Result<Response<ProblemId>, Status> {
        todo!()
    }

    async fn update(&self, req: Request<UpdateProblemRequest>) -> Result<Response<()>, Status> {
        todo!()
    }

    async fn remove(&self, req: Request<ProblemId>) -> Result<Response<()>, Status> {
        todo!()
    }

    async fn link(&self, req: Request<ProblemLink>) -> Result<Response<()>, Status> {
        todo!()
    }

    async fn unlink(&self, req: Request<ProblemLink>) -> Result<Response<()>, Status> {
        todo!()
    }

    async fn publish(&self, req: Request<ProblemId>) -> Result<Response<()>, Status> {
        todo!()
    }

    async fn unpublish(&self, req: Request<ProblemId>) -> Result<Response<()>, Status> {
        todo!()
    }

    async fn full_info_by_contest(
        &self,
        req: Request<ProblemLink>,
    ) -> Result<Response<ProblemFullInfo>, Status> {
        todo!()
    }

    async fn list_by_contest(
        &self,
        req: Request<ContestId>,
    ) -> Result<Response<ListProblemResponse>, Status> {
        todo!()
    }
}
