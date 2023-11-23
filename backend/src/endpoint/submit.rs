use super::endpoints::*;
use super::tools::*;

use super::util::stream::*;
use super::util::time::into_prost;
use crate::controller::submit::Submit;
use crate::controller::submit::SubmitBuilder;
use crate::controller::util::code::Code;
use crate::grpc::backend::submit_set_server::*;
use crate::grpc::backend::StateCode as BackendCode;
use crate::grpc::backend::*;
use crate::grpc::judger::JudgerCode;

use entity::{submit::*, *};
use tokio_stream::wrappers::ReceiverStream;

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
    ) -> Result<Response<ListSubmitResponse>, Status> {
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

        let model = Entity::read_filter(Entity::find_by_id(req.id), &auth)?
            .one(db)
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB("submit"))?;

        Ok(Response::new(model.into()))
    }

    async fn create(
        &self,
        req: Request<CreateSubmitRequest>,
    ) -> Result<Response<SubmitId>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;
        let (user_id, perm) = auth.ok_or_default()?;

        let lang = Uuid::parse_str(req.info.lang.as_str()).map_err(Into::<Error>::into)?;

        let problem = problem::Entity::find_by_id(req.info.problem_id)
            .one(db)
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB("problem"))?;

        if !problem.public {
            problem
                .find_related(contest::Entity)
                .one(db)
                .await
                .map_err(Into::<Error>::into)?
                .ok_or(Error::NotInDB("contest"))?
                .find_related(user::Entity)
                .one(db)
                .await
                .map_err(Into::<Error>::into)?
                .ok_or(Error::NotInDB("user"))?;
        }

        // if problem

        let submit = SubmitBuilder::default()
            .code(req.info.code)
            .lang(lang)
            .time_limit(problem.time)
            .memory_limit(problem.memory)
            .user(user_id)
            .problem(problem.id)
            .build()
            .unwrap();

        Ok(Response::new(self.submit.submit(submit).await?.into()))
    }

    async fn remove(&self, req: Request<SubmitId>) -> std::result::Result<Response<()>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;

        if !auth.is_root() {
            return Err(Error::PremissionDeny("only root can remove submit").into());
        }

        Entity::delete_by_id(req.id)
            .exec(db)
            .await
            .map_err(Into::<Error>::into)?;

        Ok(Response::new(()))
    }

    #[doc = " Server streaming response type for the Follow method."]
    type FollowStream = TonicStream<SubmitStatus>;

    #[doc = " are not guarantee to yield status"]
    async fn follow(&self, req: Request<SubmitId>) -> Result<Response<Self::FollowStream>, Status> {
        let (_, req) = self.parse_request(req).await?;

        Ok(Response::new(
            self.submit.follow(req.id).await.unwrap_or_else(|| {
                Box::pin(ReceiverStream::new(tokio::sync::mpsc::channel(16).1))
                    as Self::FollowStream
            }),
        ))
    }

    async fn rejudge(&self, req: Request<RejudgeRequest>) -> Result<Response<()>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;
        let (user_id, perm) = auth.ok_or_default()?;

        let submit_id = req.id.id;

        if !(perm.can_root() || perm.can_manage_submit()) {
            return Err(Error::PremissionDeny("Can't update problem").into());
        }

        let uuid = Uuid::parse_str(&req.request_id).map_err(Error::InvaildUUID)?;
        if self.dup.check(user_id, &uuid).is_some() {
            return Ok(Response::new(()));
        };

        let submit = submit::Entity::find_by_id(submit_id)
            .one(db)
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB("submit"))?;

        let problem = submit
            .find_related(problem::Entity)
            .one(db)
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB("problem"))?;

        let rejudge = SubmitBuilder::default()
            .problem(user_id)
            .problem(problem.id)
            .memory_limit(problem.memory)
            .time_limit(problem.time)
            .code(submit.code)
            .lang(Uuid::parse_str(&submit.lang).map_err(Error::InvaildUUID)?)
            .build()
            .unwrap();

        self.submit.submit(rejudge).await?;

        self.dup.store(user_id, uuid, submit_id);

        Ok(Response::new(()))
    }
}
