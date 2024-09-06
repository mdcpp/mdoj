use super::*;

use crate::controller::judger::SubmitBuilder;
use crate::util::code::Code;
use grpc::backend::{submit_server::*, StateCode as BackendCode};

use crate::entity::{
    contest, problem, submit,
    submit::{Paginator, *},
    user,
};
use tokio_stream::wrappers::ReceiverStream;

impl From<Model> for SubmitInfo {
    fn from(value: Model) -> Self {
        // TODO: solve devation and uncommitted submit!
        let db_code: Code = value.status.unwrap().try_into().unwrap();
        SubmitInfo {
            id: value.id,
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
            id: value.id,
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
impl Submit for ArcServer {
    #[instrument(
        skip_all,
        level = "info",
        name = "oj.backend.Submit/list",
        err(level = "debug", Display)
    )]
    async fn list(
        &self,
        req: Request<ListSubmitRequest>,
    ) -> Result<Response<ListSubmitResponse>, Status> {
        let (auth, req) = self.rate_limit(req).in_current_span().await?;

        req.bound_check()?;

        let paginator = match req.request.ok_or(Error::NotInPayload("request"))? {
            list_submit_request::Request::Create(create) => {
                let start_from_end = create.order == Order::Descend as i32;
                if let Some(problem_id) = create.problem_id {
                    Paginator::new_parent(problem_id, start_from_end)
                } else {
                    Paginator::new_sort(start_from_end)
                }
            }
            list_submit_request::Request::Paginator(x) => self.crypto.decode(x)?,
        };
        let mut paginator = paginator.with_auth(&auth).with_db(&self.db);

        let list = paginator.fetch(req.size, req.offset).await?;
        let remain = paginator.remain().await?;

        let paginator = paginator.into_inner();

        Ok(Response::new(ListSubmitResponse {
            list: list.into_iter().map(Into::into).collect(),
            paginator: self.crypto.encode(paginator)?,
            remain,
        }))
    }
    #[instrument(skip_all, level = "debug")]
    async fn info(&self, req: Request<Id>) -> Result<Response<SubmitInfo>, Status> {
        let (auth, req) = self.rate_limit(req).in_current_span().await?;

        debug!(id = req.id);

        let model = Entity::read_filter(Entity::find_by_id(req.id), &auth)?
            .one(self.db.deref())
            .instrument(debug_span!("fetch").or_current())
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB)?;

        Ok(Response::new(model.into()))
    }
    #[instrument(
        skip_all,
        level = "info",
        name = "oj.backend.Submit/create",
        err(level = "debug", Display)
    )]
    async fn create(&self, req: Request<CreateSubmitRequest>) -> Result<Response<Id>, Status> {
        let (auth, req) = self.rate_limit(req).in_current_span().await?;
        req.bound_check()?;

        let (user_id, _) = auth.assume_login()?;

        req.get_or_insert(|req| async move {
            let lang = Uuid::parse_str(req.lang_uid.as_str())
                .map_err(|_| Error::BadArgument("lang_uid"))?;

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

            let id = self
                .judger
                .submit(submit)
                .instrument(info_span!("construct_submit").or_current())
                .await?;

            info!(counter.submit = 1, id = id);

            Ok(id.into())
        })
        .await
        .with_grpc()
        .into()
    }

    #[instrument(
        skip_all,
        level = "info",
        name = "oj.backend.Submit/remove",
        err(level = "debug", Display)
    )]
    async fn remove(
        &self,
        req: Request<RemoveRequest>,
    ) -> std::result::Result<Response<()>, Status> {
        let (auth, req) = self.rate_limit(req).in_current_span().await?;

        req.get_or_insert(|req| async move {
            let result = Entity::delete_by_id(req.id)
                .with_auth(&auth)
                .write()?
                .exec(self.db.deref())
                .instrument(info_span!("remove").or_current())
                .await
                .map_err(Into::<Error>::into)?;

            if result.rows_affected == 0 {
                Err(Error::NotInDB)
            } else {
                info!(counter.submit = -1, id = req.id);
                Ok(())
            }
        })
        .await
        .with_grpc()
        .into()
    }

    type FollowStream = TonicStream<SubmitStatus>;

    #[instrument(
        skip_all,
        level = "info",
        name = "oj.backend.Submit/follow",
        err(level = "debug", Display)
    )]
    async fn follow(&self, req: Request<Id>) -> Result<Response<Self::FollowStream>, Status> {
        let (_, req) = self
            .parse_request_n(req, NonZeroU32!(5))
            .in_current_span()
            .await?;

        Ok(Response::new(self.judger.follow(req.id).unwrap_or_else(
            || {
                Box::pin(ReceiverStream::new(tokio::sync::mpsc::channel(16).1))
                    as Self::FollowStream
            },
        )))
    }

    #[instrument(
        skip_all,
        level = "info",
        name = "oj.backend.Submit/rejudge",
        err(level = "debug", Display)
    )]
    async fn rejudge(&self, req: Request<RejudgeRequest>) -> Result<Response<()>, Status> {
        let (auth, req) = self.rate_limit(req).in_current_span().await?;
        let (user_id, perm) = auth.assume_login()?;
        perm.admin()?;

        req.get_or_insert(|req| async move {
            let submit = submit::Entity::find_by_id(req.submit_id)
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

            Ok(())
        })
        .await
        .with_grpc()
        .into()
    }

    #[instrument(skip_all, level = "debug")]
    async fn list_lang(&self, req: Request<()>) -> Result<Response<Languages>, Status> {
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
