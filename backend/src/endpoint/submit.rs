use super::tools::*;

use crate::controller::judger::SubmitBuilder;
use crate::grpc::backend::submit_set_server::*;
use crate::grpc::backend::StateCode as BackendCode;
use crate::grpc::backend::*;
use crate::grpc::into_prost;
use crate::grpc::judger::LangInfo;
use crate::util::code::Code;

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
        let (auth, rev, size, offset, pager) = parse_pager_param!(self, req);

        let (pager, models) = match pager {
            list_submit_request::Request::Create(create) => {
                ColPaginator::new_fetch(
                    Default::default(),
                    &auth,
                    size,
                    offset,
                    create.start_from_end(),
                    &self.db,
                )
                .in_current_span()
                .await
            }
            list_submit_request::Request::Pager(old) => {
                let span = tracing::info_span!("paginate").or_current();
                let pager: ColPaginator = span.in_scope(|| self.crypto.decode(old.session))?;
                pager
                    .fetch(&auth, size, offset, rev, &self.db)
                    .instrument(span)
                    .await
            }
        }?;

        let remain = pager.remain(&auth, &self.db).in_current_span().await?;

        let next_session = self.crypto.encode(pager)?;
        let list = models.into_iter().map(|x| x.into()).collect();

        Ok(Response::new(ListSubmitResponse {
            list,
            next_session,
            remain,
        }))
    }

    #[instrument(skip_all, level = "debug")]
    async fn list_by_problem(
        &self,
        req: Request<ListByRequest>,
    ) -> Result<Response<ListSubmitResponse>, Status> {
        let (auth, rev, size, offset, pager) = parse_pager_param!(self, req);

        let (pager, models) = match pager {
            list_by_request::Request::Create(create) => {
                ParentPaginator::new_fetch(
                    (create.parent_id, Default::default()),
                    &auth,
                    size,
                    offset,
                    create.start_from_end(),
                    &self.db,
                )
                .in_current_span()
                .await
            }
            list_by_request::Request::Pager(old) => {
                let span = tracing::info_span!("paginate").or_current();
                let pager: ParentPaginator = span.in_scope(|| self.crypto.decode(old.session))?;
                pager
                    .fetch(&auth, size, offset, rev, &self.db)
                    .instrument(span)
                    .await
            }
        }?;

        let remain = pager.remain(&auth, &self.db).in_current_span().await?;

        let next_session = self.crypto.encode(pager)?;
        let list = models.into_iter().map(|x| x.into()).collect();

        Ok(Response::new(ListSubmitResponse {
            list,
            next_session,
            remain,
        }))
    }

    #[instrument(skip_all, level = "debug")]
    async fn info(&self, req: Request<SubmitId>) -> Result<Response<SubmitInfo>, Status> {
        let (auth, req) = self
            .parse_request_n(req, NonZeroU32!(5))
            .in_current_span()
            .await?;

        tracing::debug!(id = req.id);

        let model = Entity::read_filter(Entity::find_by_id(req.id), &auth)?
            .one(self.db.deref())
            .instrument(tracing::debug_span!("fetch").or_current())
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB)?;

        Ok(Response::new(model.into()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn create(
        &self,
        req: Request<CreateSubmitRequest>,
    ) -> Result<Response<SubmitId>, Status> {
        let (auth, req) = self.parse_request_n(req, crate::NonZeroU32!(15)).await?;
        let (user_id, _) = auth.ok_or_default()?;

        if req.code.len() > SUBMIT_CODE_LEN {
            return Err(Error::BufferTooLarge("info.code").into());
        }

        let lang = Uuid::parse_str(req.lang.as_str()).map_err(Into::<Error>::into)?;

        let problem = problem::Entity::find_by_id(req.problem_id)
            .one(self.db.deref())
            .instrument(info_span!("fetch_problem").or_current())
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB)?;

        if (problem.user_id != user_id) && (!problem.public) {
            problem
                .find_related(contest::Entity)
                .one(self.db.deref())
                .instrument(info_span!("fetch_contest").or_current())
                .await
                .map_err(Into::<Error>::into)?
                .ok_or(Error::NotInDB)?
                .find_related(user::Entity)
                .one(self.db.deref())
                .instrument(info_span!("fetch_user").or_current())
                .await
                .map_err(Into::<Error>::into)?
                .ok_or(Error::NotInDB)?;
        }

        let submit = SubmitBuilder::default()
            .code(req.code)
            .lang(lang)
            .time_limit(problem.time)
            .memory_limit(problem.memory)
            .user(user_id)
            .problem(problem.id)
            .build()
            .unwrap();

        info!(msg = "submit has been created, not judged nor committed yet.");

        let id = self
            .judger
            .submit(submit)
            .instrument(info_span!("construct_submit").or_current())
            .await?;

        tracing::debug!(id = id, "submit_created");
        self.metrics.submit(1);

        Ok(Response::new(id.into()))
    }

    #[instrument(skip_all, level = "debug")]
    async fn remove(&self, req: Request<SubmitId>) -> std::result::Result<Response<()>, Status> {
        let (auth, req) = self
            .parse_request_n(req, NonZeroU32!(5))
            .in_current_span()
            .await?;

        let result = Entity::write_filter(Entity::delete_by_id(req.id), &auth)?
            .exec(self.db.deref())
            .instrument(info_span!("remove").or_current())
            .await
            .map_err(Into::<Error>::into)?;

        if result.rows_affected == 0 {
            return Err(Error::NotInDB.into());
        }

        tracing::debug!(id = req.id);
        self.metrics.submit(-1);

        Ok(Response::new(()))
    }

    #[doc = " Server streaming response type for the Follow method."]
    type FollowStream = TonicStream<SubmitStatus>;

    #[doc = " are not guarantee to yield status"]
    #[instrument(skip_all, level = "debug")]
    async fn follow(&self, req: Request<SubmitId>) -> Result<Response<Self::FollowStream>, Status> {
        let (_, req) = self
            .parse_request_n(req, NonZeroU32!(5))
            .in_current_span()
            .await?;

        tracing::trace!(id = req.id);

        Ok(Response::new(self.judger.follow(req.id).unwrap_or_else(
            || {
                Box::pin(ReceiverStream::new(tokio::sync::mpsc::channel(16).1))
                    as Self::FollowStream
            },
        )))
    }

    #[instrument(skip_all, level = "debug")]
    async fn rejudge(&self, req: Request<RejudgeRequest>) -> Result<Response<()>, Status> {
        let (auth, req) = self
            .parse_request_n(req, NonZeroU32!(5))
            .in_current_span()
            .await?;
        let (user_id, perm) = auth.ok_or_default()?;

        let submit_id = req.id.id;

        if !(perm.super_user()) {
            return Err(Error::RequirePermission(RoleLv::Super).into());
        }

        let uuid = Uuid::parse_str(&req.request_id).map_err(Error::InvaildUUID)?;
        if let Some(x) = self.dup.check::<()>(user_id, uuid) {
            return Ok(Response::new(x));
        };

        tracing::debug!(req.id = submit_id);

        let submit = submit::Entity::find_by_id(submit_id)
            .one(self.db.deref())
            .instrument(info_span!("fetch_submit").or_current())
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB)?;

        let problem = submit
            .find_related(problem::Entity)
            .one(self.db.deref())
            .instrument(info_span!("fetch_problem").or_current())
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB)?;

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

        self.judger
            .submit(rejudge)
            .instrument(info_span!("construct_submit").or_current())
            .await?;

        self.dup.store(user_id, uuid, ());

        Ok(Response::new(()))
    }

    #[instrument(skip_all, level = "debug")]
    async fn list_langs(&self, req: Request<()>) -> Result<Response<Languages>, Status> {
        self.parse_request_n(req, NonZeroU32!(5))
            .in_current_span()
            .await?;

        let list: Vec<_> = self
            .judger
            .list_lang()
            .into_iter()
            .map(|x| x.into())
            .collect();

        Ok(Response::new(Languages { list }))
    }
}
