use super::tools::*;
use grpc::backend::problem_server::*;
use grpc::backend::*;

use crate::entity::{problem::*, *};

impl From<PartialModel> for ProblemInfo {
    fn from(value: PartialModel) -> Self {
        ProblemInfo {
            id: value.id.into(),
            title: value.title,
            submit_count: value.submit_count,
            ac_rate: value.ac_rate,
            difficulty: value.difficulty,
        }
    }
}

impl From<Model> for ProblemFullInfo {
    fn from(value: Model) -> Self {
        ProblemFullInfo {
            content: value.content.clone(),
            tags: value.tags.clone(),
            difficulty: value.difficulty,
            public: value.public,
            time: value.time as u64,
            memory: value.memory as u64,
            info: ProblemInfo {
                id: value.id.into(),
                title: value.title,
                submit_count: value.submit_count,
                ac_rate: value.ac_rate,
                difficulty: value.difficulty,
            },
            author: value.user_id.into(),
        }
    }
}

#[async_trait]
impl Problem for ArcServer {
    #[instrument(skip_all, level = "debug")]
    async fn list(
        &self,
        req: Request<ListProblemRequest>,
    ) -> Result<Response<ListProblemResponse>, Status> {
        todo!()
    }
    #[instrument(skip_all, level = "debug")]
    async fn full_info(&self, req: Request<Id>) -> Result<Response<ProblemFullInfo>, Status> {
        let (auth, req) = self
            .parse_request_n(req, NonZeroU32!(5))
            .in_current_span()
            .await?;

        debug!(problem_id = req.id);

        let query = Entity::read_filter(Entity::find_by_id::<i32>(req.into()), &auth)?;
        let model = query
            .one(self.db.deref())
            .instrument(info_span!("fetch"))
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB)?;

        Ok(Response::new(model.into()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn create(&self, req: Request<CreateProblemRequest>) -> Result<Response<Id>, Status> {
        let (auth, req) = self
            .parse_request_n(req, NonZeroU32!(5))
            .in_current_span()
            .await?;
        let (user_id, perm) = auth.auth_or_guest()?;

        req.check_with_error()?;

        let uuid = Uuid::parse_str(&req.request_id).map_err(Error::InvaildUUID)?;
        if let Some(x) = self.dup.check::<Id>(user_id, uuid) {
            return Ok(Response::new(x));
        };

        if !perm.super_user() {
            return Err(Error::RequirePermission(RoleLv::Super).into());
        }

        if req.info.memory > 2 * 1024 * 1024 * 1024 || req.info.time > 10 * 1000 * 1000 {
            return Err(Error::NumberTooLarge.into());
        }

        let mut model: ActiveModel = Default::default();
        model.user_id = ActiveValue::Set(user_id);

        fill_active_model!(
            model, req.info, title, difficulty, time, memory, tags, content, match_rule, order
        );

        let model = model
            .save(self.db.deref())
            .instrument(info_span!("save").or_current())
            .await
            .map_err(Into::<Error>::into)?;

        let id: Id = model.id.clone().unwrap().into();

        self.dup.store(user_id, uuid, id.clone());

        tracing::debug!(id = id.id);

        Ok(Response::new(id))
    }
    #[instrument(skip_all, level = "debug")]
    async fn update(&self, req: Request<UpdateProblemRequest>) -> Result<Response<()>, Status> {
        let (auth, req) = self
            .parse_request_n(req, NonZeroU32!(5))
            .in_current_span()
            .await?;
        let (user_id, _perm) = auth.auth_or_guest()?;

        req.check_with_error()?;

        let uuid = Uuid::parse_str(&req.request_id).map_err(Error::InvaildUUID)?;
        if let Some(x) = self.dup.check::<()>(user_id, uuid) {
            return Ok(Response::new(x));
        };

        tracing::trace!(id = req.id);

        let mut model = Entity::write_filter(Entity::find_by_id(req.id), &auth)?
            .one(self.db.deref())
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB)?
            .into_active_model();

        fill_exist_active_model!(
            model, req.info, title, difficulty, time, memory, tags, content, match_rule, order
        );

        model
            .update(self.db.deref())
            .instrument(info_span!("update").or_current())
            .await
            .map_err(atomic_fail)?;

        self.dup.store(user_id, uuid, ());

        Ok(Response::new(()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn remove(&self, req: Request<Id>) -> Result<Response<()>, Status> {
        let (auth, req) = self
            .parse_request_n(req, NonZeroU32!(5))
            .in_current_span()
            .await?;

        let result = Entity::write_filter(Entity::delete_by_id(Into::<i32>::into(req.id)), &auth)?
            .exec(self.db.deref())
            .instrument(info_span!("remove").or_current())
            .await
            .map_err(Into::<Error>::into)?;

        if result.rows_affected == 0 {
            return Err(Error::NotInDB.into());
        }

        tracing::debug!(id = req.id);

        Ok(Response::new(()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn add_to_contest(
        &self,
        req: Request<AddProblemToContestRequest>,
    ) -> Result<Response<()>, Status> {
        let (auth, req) = self
            .parse_request_n(req, NonZeroU32!(5))
            .in_current_span()
            .await?;
        let (user_id, perm) = auth.auth_or_guest()?;

        if !perm.admin() {
            return Err(Error::RequirePermission(RoleLv::Root).into());
        }

        let (contest, model) = tokio::try_join!(
            contest::Entity::read_by_id(req.contest_id, &auth)?
                .one(self.db.deref())
                .instrument(debug_span!("find_parent").or_current()),
            Entity::read_by_id(req.problem_id, &auth)?
                .one(self.db.deref())
                .instrument(debug_span!("find_child").or_current())
        )
        .map_err(Into::<Error>::into)?;

        let contest = contest.ok_or(Error::NotInDB)?;
        let model = model.ok_or(Error::NotInDB)?;

        if !perm.admin() {
            if contest.hoster != user_id {
                return Err(Error::NotInDB.into());
            }
            if model.user_id != user_id {
                return Err(Error::NotInDB.into());
            }
        }

        let mut model = model.into_active_model();
        if let Some(x) = model.contest_id.into_value() {
            tracing::debug!(old_id = x.to_string());
        }
        model.contest_id = ActiveValue::Set(Some(req.problem_id));
        model
            .save(self.db.deref())
            .instrument(info_span!("update_child").or_current())
            .await
            .map_err(Into::<Error>::into)?;

        Ok(Response::new(()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn remove_from_contest(
        &self,
        req: Request<AddProblemToContestRequest>,
    ) -> Result<Response<()>, Status> {
        let (auth, req) = self
            .parse_request_n(req, NonZeroU32!(5))
            .in_current_span()
            .await?;

        let mut problem = Entity::write_by_id(req.problem_id, &auth)?
            .columns([Column::Id, Column::ContestId])
            .one(self.db.deref())
            .instrument(info_span!("fetch").or_current())
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB)?
            .into_active_model();

        problem.contest_id = ActiveValue::Set(None);

        problem
            .update(self.db.deref())
            .instrument(info_span!("update").or_current())
            .await
            .map_err(atomic_fail)?;

        Ok(Response::new(()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn publish(&self, req: Request<Id>) -> Result<Response<()>, Status> {
        let (auth, req) = self
            .parse_request_n(req, NonZeroU32!(5))
            .in_current_span()
            .await?;
        let (_, perm) = auth.auth_or_guest()?;

        tracing::debug!(id = req.id);

        let mut query = Entity::find_by_id(Into::<i32>::into(req));

        if !perm.admin() {
            query = Entity::write_filter(query, &auth)?;
        }

        let mut problem = query
            .columns([Column::Id, Column::ContestId])
            .one(self.db.deref())
            .instrument(info_span!("fetch").or_current())
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB)?
            .into_active_model();

        problem.public = ActiveValue::Set(true);

        problem
            .update(self.db.deref())
            .instrument(info_span!("update").or_current())
            .await
            .map_err(atomic_fail)?;

        Ok(Response::new(()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn unpublish(&self, req: Request<Id>) -> Result<Response<()>, Status> {
        let (auth, req) = self
            .parse_request_n(req, NonZeroU32!(5))
            .in_current_span()
            .await?;
        let (_, perm) = auth.auth_or_guest()?;

        tracing::debug!(id = req.id);

        let mut query = Entity::find_by_id(Into::<i32>::into(req));

        if !perm.super_user() {
            query = Entity::write_filter(query, &auth)?;
        }

        let mut problem = query
            .columns([Column::Id, Column::ContestId])
            .one(self.db.deref())
            .instrument(info_span!("fetch").or_current())
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB)?
            .into_active_model();

        problem.public = ActiveValue::Set(false);

        problem
            .update(self.db.deref())
            .instrument(info_span!("update").or_current())
            .await
            .map_err(atomic_fail)?;

        Ok(Response::new(()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn full_info_by_contest(
        &self,
        req: Request<AddProblemToContestRequest>,
    ) -> Result<Response<ProblemFullInfo>, Status> {
        let (auth, req) = self
            .parse_request_n(req, NonZeroU32!(5))
            .in_current_span()
            .await?;

        let parent: contest::IdModel =
            contest::Entity::related_read_by_id(&auth, Into::<i32>::into(req.contest_id), &self.db)
                .in_current_span()
                .await?;

        let model = parent
            .upgrade()
            .find_related(Entity)
            .filter(Column::Id.eq(Into::<i32>::into(req.problem_id)))
            .one(self.db.deref())
            .instrument(info_span!("fetch").or_current())
            .in_current_span()
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB)?;

        Ok(Response::new(model.into()))
    }
}
