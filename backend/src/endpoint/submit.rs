use super::tools::*;

use crate::controller::code::Code;
use crate::controller::judger::SubmitBuilder;
use crate::grpc::backend::submit_set_server::*;
use crate::grpc::backend::StateCode as BackendCode;
use crate::grpc::backend::*;
use crate::grpc::into_prost;
use crate::grpc::judger::LangInfo;

use crate::entity::{submit::*, *};
use tokio_stream::wrappers::ReceiverStream;

const SUBMIT_CODE_LEN: usize = 32 * 1024;

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
        // TODO: solve devation and uncommitted submit!
        let db_code: Code = value.status.unwrap().try_into().unwrap();
        SubmitInfo {
            id: value.id.into(),
            upload_time: into_prost(value.upload_at),
            score: value.score,
            state: JudgeResult {
                code: Into::<BackendCode>::into(db_code).into(),
                accuracy: value.accuracy.map(|x| x as u64),
                time: value.time.map(|x| x as u64),
                memory: value.memory.map(|x| x as u64),
            },
        }
    }
}

impl From<PartialModel> for SubmitInfo {
    fn from(value: PartialModel) -> Self {
        // TODO: solve devation and uncommitted submit!
        let db_code: Code = value.status.unwrap().try_into().unwrap();
        SubmitInfo {
            id: value.id.into(),
            upload_time: into_prost(value.upload_at),
            score: value.score,
            state: JudgeResult {
                code: Into::<BackendCode>::into(db_code).into(),
                accuracy: value.accuracy.map(|x| x as u64),
                time: value.time.map(|x| x as u64),
                memory: value.memory.map(|x| x as u64),
            },
        }
    }
}

#[async_trait]
impl SubmitSet for Arc<Server> {
    #[instrument(skip_all, level = "debug")]
    async fn list(
        &self,
        req: Request<ListSubmitRequest>,
    ) -> Result<Response<ListSubmitResponse>, Status> {
        let (auth, req) = self.parse_request(req).await?;
        let size = req.size;
        let offset = req.offset();

        let (pager, models) = match req.request.ok_or(Error::NotInPayload("request"))? {
            list_submit_request::Request::Create(_create) => {
                ColPaginator::new_fetch(Default::default(), &auth, size, offset, true).await
            }
            list_submit_request::Request::Pager(old) => {
                let pager: ColPaginator = self.crypto.decode(old.session)?;
                pager.fetch(&auth, size, offset, old.reverse).await
            }
        }?;

        let next_session = self.crypto.encode(pager)?;
        let list = models.into_iter().map(|x| x.into()).collect();

        Ok(Response::new(ListSubmitResponse { list, next_session }))
    }

    #[instrument(skip_all, level = "debug")]
    async fn list_by_problem(
        &self,
        req: Request<ListByRequest>,
    ) -> Result<Response<ListSubmitResponse>, Status> {
        let (auth, req) = self.parse_request(req).await?;
        let size = req.size;
        let offset = req.offset();

        let (pager, models) = match req.request.ok_or(Error::NotInPayload("request"))? {
            list_by_request::Request::Create(create) => {
                ParentPaginator::new_fetch(
                    (create.parent_id, Default::default()),
                    &auth,
                    size,
                    offset,
                    create.start_from_end,
                )
                .await
            }
            list_by_request::Request::Pager(old) => {
                let pager: ParentPaginator = self.crypto.decode(old.session)?;
                pager.fetch(&auth, size, offset, old.reverse).await
            }
        }?;

        let next_session = self.crypto.encode(pager)?;
        let list = models.into_iter().map(|x| x.into()).collect();

        Ok(Response::new(ListSubmitResponse { list, next_session }))
    }

    #[instrument(skip_all, level = "debug")]
    async fn info(&self, req: Request<SubmitId>) -> Result<Response<SubmitInfo>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;

        tracing::debug!(id = req.id);

        let model = Entity::read_filter(Entity::find_by_id(req.id), &auth)?
            .one(db)
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB(Entity::DEBUG_NAME))?;

        Ok(Response::new(model.into()))
    }

    #[instrument(skip_all, level = "debug")]
    async fn create(
        &self,
        req: Request<CreateSubmitRequest>,
    ) -> Result<Response<SubmitId>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;
        let (user_id, _) = auth.ok_or_default()?;

        if req.info.code.len() > SUBMIT_CODE_LEN {
            return Err(Error::BufferTooLarge("info.code").into());
        }

        let lang = Uuid::parse_str(req.info.lang.as_str()).map_err(Into::<Error>::into)?;

        let problem = problem::Entity::find_by_id(req.info.problem_id)
            .one(db)
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB("problem"))?;

        if (problem.user_id != user_id) && (!problem.public) {
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

        let submit = SubmitBuilder::default()
            .code(req.info.code)
            .lang(lang)
            .time_limit(problem.time)
            .memory_limit(problem.memory)
            .user(user_id)
            .problem(problem.id)
            .build()
            .unwrap();

        let id = self.judger.submit(submit).await?;

        tracing::debug!(id = id, "submit_created");
        self.metrics.submit.add(1, &[]);

        Ok(Response::new(id.into()))
    }

    #[instrument(skip_all, level = "debug")]
    async fn remove(&self, req: Request<SubmitId>) -> std::result::Result<Response<()>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;

        let result = Entity::write_filter(Entity::delete_by_id(req.id), &auth)?
            .exec(db)
            .await
            .map_err(Into::<Error>::into)?;

        if result.rows_affected == 0 {
            return Err(Error::NotInDB(Entity::DEBUG_NAME).into());
        }

        tracing::debug!(id = req.id);
        self.metrics.submit.add(-1, &[]);

        Ok(Response::new(()))
    }

    #[doc = " Server streaming response type for the Follow method."]
    type FollowStream = TonicStream<SubmitStatus>;

    #[doc = " are not guarantee to yield status"]
    #[instrument(skip_all, level = "debug")]
    async fn follow(&self, req: Request<SubmitId>) -> Result<Response<Self::FollowStream>, Status> {
        let (_, req) = self.parse_request(req).await?;

        tracing::trace!(id = req.id);

        Ok(Response::new(
            self.judger.follow(req.id).await.unwrap_or_else(|| {
                Box::pin(ReceiverStream::new(tokio::sync::mpsc::channel(16).1))
                    as Self::FollowStream
            }),
        ))
    }

    #[instrument(skip_all, level = "debug")]
    async fn rejudge(&self, req: Request<RejudgeRequest>) -> Result<Response<()>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;
        let (user_id, perm) = auth.ok_or_default()?;

        let submit_id = req.id.id;

        if !(perm.super_user()) {
            return Err(Error::RequirePermission(RoleLv::Super).into());
        }

        let uuid = Uuid::parse_str(&req.request_id).map_err(Error::InvaildUUID)?;
        if self.dup.check_i32(user_id, &uuid).is_some() {
            return Ok(Response::new(()));
        };

        tracing::debug!(req.id = submit_id);

        let submit = submit::Entity::find_by_id(submit_id)
            .one(db)
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB(Entity::DEBUG_NAME))?;

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
            .user(user_id)
            .lang(Uuid::parse_str(&submit.lang).map_err(Error::InvaildUUID)?)
            .build()
            .unwrap();

        self.judger.submit(rejudge).await?;

        self.dup.store_i32(user_id, uuid, submit_id);

        Ok(Response::new(()))
    }

    #[doc = " Server streaming response type for the ListLangs method."]
    type ListLangsStream = TonicStream<Language>;

    #[instrument(skip_all, level = "debug")]
    async fn list_langs(&self, _: Request<()>) -> Result<Response<Self::ListLangsStream>, Status> {
        let langs = self.judger.list_lang().into_iter().map(|x| Ok(x.into()));

        Ok(Response::new(
            Box::pin(tokio_stream::iter(langs)) as TonicStream<_>
        ))
    }
}

impl From<LangInfo> for Language {
    fn from(value: LangInfo) -> Self {
        Language {
            lang_uid: value.lang_uid,
            lang_name: value.lang_name,
            info: value.info,
            lang_ext: value.lang_ext,
        }
    }
}
