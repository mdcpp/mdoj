use super::tools::*;
use tracing::Instrument;

use crate::grpc::backend::problem_set_server::*;
use crate::grpc::backend::*;

use crate::entity::{problem::*, *};

impl From<i32> for ProblemId {
    fn from(value: i32) -> Self {
        ProblemId { id: value }
    }
}

impl From<ProblemId> for i32 {
    fn from(value: ProblemId) -> Self {
        value.id
    }
}

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
impl ProblemSet for Arc<Server> {
    #[instrument(skip_all, level = "debug")]
    async fn list(
        &self,
        req: Request<ListProblemRequest>,
    ) -> Result<Response<ListProblemResponse>, Status> {
        let (auth, rev, size, offset, pager) = parse_pager_param!(self, req);

        let (pager, models) = match pager {
            list_problem_request::Request::Create(create) => {
                ColPaginator::new_fetch(
                    (create.sort_by(), Default::default()),
                    &auth,
                    size,
                    offset,
                    create.start_from_end(),
                    &self.db,
                )
                .in_current_span()
                .await
            }
            list_problem_request::Request::Pager(old) => {
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

        Ok(Response::new(ListProblemResponse {
            list,
            next_session,
            remain,
        }))
    }
    #[instrument(skip_all, level = "debug")]
    async fn list_by_contest(
        &self,
        req: Request<ListByRequest>,
    ) -> Result<Response<ListProblemResponse>, Status> {
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

        Ok(Response::new(ListProblemResponse {
            list,
            next_session,
            remain,
        }))
    }
    #[instrument(skip_all, level = "debug")]
    async fn search_by_text(
        &self,
        req: Request<TextSearchRequest>,
    ) -> Result<Response<ListProblemResponse>, Status> {
        let (auth, rev, size, offset, pager) = parse_pager_param!(self, req);

        let (pager, models) = match pager {
            text_search_request::Request::Text(text) => {
                TextPaginator::new_fetch(text, &auth, size, offset, true, &self.db)
                    .in_current_span()
                    .await
            }
            text_search_request::Request::Pager(old) => {
                let span = tracing::info_span!("paginate").or_current();
                let pager: TextPaginator = span.in_scope(|| self.crypto.decode(old.session))?;
                pager
                    .fetch(&auth, size, offset, rev, &self.db)
                    .instrument(span)
                    .await
            }
        }?;

        let remain = pager.remain(&auth, &self.db).in_current_span().await?;

        let next_session = self.crypto.encode(pager)?;
        let list = models.into_iter().map(|x| x.into()).collect();

        Ok(Response::new(ListProblemResponse {
            list,
            next_session,
            remain,
        }))
    }
    #[instrument(skip_all, level = "debug")]
    async fn full_info(
        &self,
        req: Request<ProblemId>,
    ) -> Result<Response<ProblemFullInfo>, Status> {
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
    async fn create(
        &self,
        req: Request<CreateProblemRequest>,
    ) -> Result<Response<ProblemId>, Status> {
        let (auth, req) = self
            .parse_request_n(req, NonZeroU32!(5))
            .in_current_span()
            .await?;
        let (user_id, perm) = auth.ok_or_default()?;

        check_length!(SHORT_ART_SIZE, req.info, title, tags);
        check_length!(LONG_ART_SIZE, req.info, content);

        let uuid = Uuid::parse_str(&req.request_id).map_err(Error::InvaildUUID)?;
        if let Some(x) = self.dup.check::<ProblemId>(user_id, uuid) {
            return Ok(Response::new(x));
        };

        if !(perm.super_user()) {
            return Err(Error::RequirePermission(RoleLv::Super).into());
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

        let id: ProblemId = model.id.clone().unwrap().into();

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
        let (user_id, _perm) = auth.ok_or_default()?;

        check_exist_length!(SHORT_ART_SIZE, req.info, title, tags);
        check_exist_length!(LONG_ART_SIZE, req.info, content);

        let uuid = Uuid::parse_str(&req.request_id).map_err(Error::InvaildUUID)?;
        if let Some(x) = self.dup.check::<()>(user_id, uuid) {
            return Ok(Response::new(x));
        };

        tracing::trace!(id = req.id.id);

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
            .map_err(Into::<Error>::into)?;

        self.dup.store(user_id, uuid, ());

        Ok(Response::new(()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn remove(&self, req: Request<ProblemId>) -> Result<Response<()>, Status> {
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
        let (user_id, perm) = auth.ok_or_default()?;

        if !perm.admin() {
            return Err(Error::RequirePermission(RoleLv::Root).into());
        }

        let (contest, model) = tokio::try_join!(
            contest::Entity::read_by_id(req.contest_id.id, &auth)?
                .one(self.db.deref())
                .instrument(debug_span!("find_parent").or_current()),
            Entity::read_by_id(req.problem_id.id, &auth)?
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
        model.contest_id = ActiveValue::Set(Some(req.problem_id.id));
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
            .save(self.db.deref())
            .instrument(info_span!("update").or_current())
            .await
            .map_err(Into::<Error>::into)?;

        Ok(Response::new(()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn publish(&self, req: Request<ProblemId>) -> Result<Response<()>, Status> {
        let (auth, req) = self
            .parse_request_n(req, NonZeroU32!(5))
            .in_current_span()
            .await?;
        let (_, perm) = auth.ok_or_default()?;

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
            .save(self.db.deref())
            .instrument(info_span!("update").or_current())
            .await
            .map_err(Into::<Error>::into)?;

        Ok(Response::new(()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn unpublish(&self, req: Request<ProblemId>) -> Result<Response<()>, Status> {
        let (auth, req) = self
            .parse_request_n(req, NonZeroU32!(5))
            .in_current_span()
            .await?;
        let (_, perm) = auth.ok_or_default()?;

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
            .save(self.db.deref())
            .instrument(info_span!("update").or_current())
            .await
            .map_err(Into::<Error>::into)?;

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
