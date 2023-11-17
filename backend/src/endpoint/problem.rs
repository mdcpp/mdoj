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
impl Filter for Entity {
    async fn read_filter(query: Select<Self>, auth: &Auth) -> Result<Select<Self>, Error> {
        if let Some(perm) = auth.user_perm() {
            if perm.can_link() || perm.can_root() || perm.can_manage_problem() {
                return Ok(query);
            }
        }
        Ok(query.filter(Column::Public.eq(true)))
    }
    async fn write_filter(query: Select<Self>, auth: &Auth) -> Result<Select<Self>, Error> {
        if let Some(perm) = auth.user_perm() {
            if perm.can_root() {
                return Ok(query);
            }
            if perm.can_manage_problem() {
                let user_id = auth.user_id().unwrap();
                return Ok(query.filter(Column::UserId.eq(user_id)));
            }
        }
        Ok(query.filter(Column::Id.eq(-1)))
    }
}

impl Into<ProblemId> for i32 {
    fn into(self) -> ProblemId {
        ProblemId { id: self }
    }
}

impl From<ProblemId> for i32 {
    fn from(value: ProblemId) -> Self {
        value.id
    }
}

impl Into<ProblemInfo> for Model {
    fn into(self) -> ProblemInfo {
        ProblemInfo {
            id: self.id.into(),
            title: self.title,
            submit_count: self.submit_count,
            ac_rate: self.ac_rate,
        }
    }
}

#[tonic::async_trait]
impl ProblemSet for Server {
    async fn list(
        &self,
        req: Request<ListRequest>,
    ) -> Result<Response<ListProblemResponse>, Status> {
        let (auth, req) = self.parse_request(req).await.i()?;

        let mut reverse = false;
        let mut pager: Pager<Entity> =
            match req.request.ok_or(Error::NotInPayload("request")).i()? {
                list_request::Request::Create(create) => {
                    Pager::sort_search(create.sort_by(), create.reverse)
                }
                list_request::Request::Pager(old) => {
                    reverse = old.reverse;
                    <Pager<Entity> as HasParentPager<contest::Entity, Entity>>::from_raw(
                        old.session,
                    )
                    .i()?
                }
            };

        let list = pager
            .fetch(req.size, reverse, &auth)
            .await
            .i()?
            .into_iter()
            .map(|x| x.into())
            .collect();

        let next_page_token = pager.into_raw();

        Ok(Response::new(ListProblemResponse {
            list,
            next_page_token,
        }))
    }

    async fn search_by_text(
        &self,
        req: Request<TextSearchRequest>,
    ) -> Result<Response<ListProblemResponse>, Status> {
        let (auth, req) = self.parse_request(req).await.i()?;

        let mut reverse = false;
        let mut pager: Pager<Entity> =
            match req.request.ok_or(Error::NotInPayload("request")).i()? {
                text_search_request::Request::Text(create) => Pager::text_search(create),
                text_search_request::Request::Pager(old) => {
                    reverse = old.reverse;
                    <Pager<Entity> as HasParentPager<contest::Entity, Entity>>::from_raw(
                        old.session,
                    )
                    .i()?
                }
            };

        let list = pager
            .fetch(req.size, reverse, &auth)
            .await
            .i()?
            .into_iter()
            .map(|x| x.into())
            .collect();

        let next_page_token = pager.into_raw();

        Ok(Response::new(ListProblemResponse {
            list,
            next_page_token,
        }))
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
