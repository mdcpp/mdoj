use super::endpoints::*;
use super::tools::*;

use super::util::stream::*;
use super::util::time::into_prost;
use crate::controller::util::code::Code;
use crate::grpc::backend::submit_set_server::*;
use crate::grpc::backend::StateCode as BackendCode;
use crate::grpc::backend::*;
use crate::grpc::judger::JudgerCode;

use entity::{submit::*, *};

impl Filter for Entity {
    fn read_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
        Ok(query)
    }

    fn write_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
        if let Some(perm) = auth.user_perm() {
            if perm.can_manage_submit() || perm.can_root() {
                return Ok(query);
            }
        }
        Err(Error::Unauthenticated)
    }
}

impl From<i32> for SubmitId {
    fn from(value: i32) -> Self {
        SubmitId { id: value }
    }
}

impl From<SubmitId> for i32 {
    fn from(value: SubmitId) -> Self {
        value.id
    }
}

impl From<Model> for SubmitInfo {
    fn from(value: Model) -> Self {
        // TODO: solve devation aand uncommitted submit!
        let db_code: Code = value.status.try_into().unwrap();
        SubmitInfo {
            id: value.id.into(),
            upload_time: into_prost(value.upload_at),
            score: value.score,
            state: JudgeResult {
                code: Into::<BackendCode>::into(db_code).into(),
                accuracy: value.accuracy,
                time: value.time,
                memory: value.memory,
            },
        }
    }
}

#[async_trait]
impl SubmitSet for Arc<Server> {
    async fn list(
        &self,
        req: Request<ListRequest>,
    ) -> Result<Response<ListSubmitResponse>, Status> {
        let (auth, req) = self.parse_request(req).await?;

        let mut reverse = false;
        let mut pager: Pager<Entity> = match req.request.ok_or(Error::NotInPayload("request"))? {
            list_request::Request::Create(create) => {
                Pager::sort_search(create.sort_by(), create.reverse)
            }
            list_request::Request::Pager(old) => {
                reverse = old.reverse;
                <Pager<Entity> as HasParentPager<problem::Entity, Entity>>::from_raw(old.session)?
            }
        };

        let list = pager
            .fetch(req.size, req.offset.unwrap_or_default(), reverse, &auth)
            .await?
            .into_iter()
            .map(|x| x.into())
            .collect();

        let next_session = pager.into_raw();

        Ok(Response::new(ListSubmitResponse { list, next_session }))
    }

    async fn list_by_problem(
        &self,
        req: Request<ListByRequest>,
    ) -> Result<Response<ListSubmitResponse>, tonic::Status> {
        let (auth, req) = self.parse_request(req).await?;

        let mut reverse = false;
        let mut pager: Pager<Entity> = match req.request.ok_or(Error::NotInPayload("request"))? {
            list_by_request::Request::ParentId(ppk) => Pager::parent_search(ppk),
            list_by_request::Request::Pager(old) => {
                reverse = old.reverse;
                <Pager<Entity> as HasParentPager<problem::Entity, Entity>>::from_raw(old.session)?
            }
        };

        let list = pager
            .fetch(req.size, req.offset.unwrap_or_default(), reverse, &auth)
            .await?
            .into_iter()
            .map(|x| x.into())
            .collect();

        let next_session = pager.into_raw();

        Ok(Response::new(ListSubmitResponse { list, next_session }))
    }

    async fn info(&self, req: Request<SubmitId>) -> Result<Response<SubmitInfo>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;

        let model=Entity::read_filter(Entity::find_by_id(req.id), &auth)?
            .one(db)
            .await.map_err(Into::<Error>::into)?.ok_or(Error::NotInDB("submit"))?;

        Ok(Response::new(model.into()))
    }

    async fn create(
        &self,
        req: Request<CreateSubmitRequest>,
    ) -> Result<Response<SubmitId>, Status> {
        todo!()
    }

    async fn remove(
        &self,
        req: Request<SubmitId>,
    ) -> std::result::Result<tonic::Response<()>, Status> {
        todo!()
    }

    #[doc = " Server streaming response type for the Follow method."]
    type FollowStream = TonicStream<SubmitStatus>;

    #[doc = " are not guarantee to yield status"]
    async fn follow(
        &self,
        req: Request<SubmitId>,
    ) -> std::result::Result<tonic::Response<Self::FollowStream>, Status> {
        todo!()
    }

    async fn rejudge(
        &self,
        req: Request<RejudgeRequest>,
    ) -> std::result::Result<tonic::Response<()>, Status> {
        todo!()
    }
}
