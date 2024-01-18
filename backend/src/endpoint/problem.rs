use super::tools::*;

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

impl From<Model> for ProblemInfo {
    fn from(value: Model) -> Self {
        ProblemInfo {
            id: value.id.into(),
            title: value.title,
            submit_count: value.submit_count,
            ac_rate: value.ac_rate,
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
            info: value.into(),
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
        let (auth, req) = self.parse_request(req).await?;

        let mut reverse = false;
        let mut pager: Pager<Entity> = match req.request.ok_or(Error::NotInPayload("request"))? {
            list_problem_request::Request::Create(create) => {
                Pager::sort_search(create.sort_by(), create.reverse)
            }
            list_problem_request::Request::Pager(old) => {
                reverse = old.reverse;
                <Pager<Entity> as HasParentPager<contest::Entity, Entity>>::from_raw(
                    old.session,
                    self,
                )?
            }
        };

        let list = pager
            .fetch(req.size, req.offset.unwrap_or_default(), reverse, &auth)
            .await?
            .into_iter()
            .map(|x| x.into())
            .collect();

        let next_session = pager.into_raw(self);

        Ok(Response::new(ListProblemResponse { list, next_session }))
    }
    #[instrument(skip_all, level = "debug")]
    async fn search_by_text(
        &self,
        req: Request<TextSearchRequest>,
    ) -> Result<Response<ListProblemResponse>, Status> {
        let (auth, req) = self.parse_request(req).await?;

        let mut reverse = false;
        let mut pager: Pager<Entity> = match req.request.ok_or(Error::NotInPayload("request"))? {
            text_search_request::Request::Text(create) => {
                tracing::trace!(search = create);
                Pager::text_search(create)
            }
            text_search_request::Request::Pager(old) => {
                reverse = old.reverse;
                <Pager<_> as HasParentPager<contest::Entity, Entity>>::from_raw(old.session, self)?
            }
        };

        let list = pager
            .fetch(req.size, req.offset.unwrap_or_default(), reverse, &auth)
            .await?
            .into_iter()
            .map(|x| x.into())
            .collect();

        let next_session = pager.into_raw(self);

        Ok(Response::new(ListProblemResponse { list, next_session }))
    }
    #[instrument(skip_all, level = "debug")]
    async fn full_info(
        &self,
        req: Request<ProblemId>,
    ) -> Result<Response<ProblemFullInfo>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;

        tracing::debug!(problem_id = req.id);

        let query = Entity::read_filter(Entity::find_by_id::<i32>(req.into()), &auth)?;
        let model = query
            .one(db)
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB(Entity::DEBUG_NAME))?;

        Ok(Response::new(model.into()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn create(
        &self,
        req: Request<CreateProblemRequest>,
    ) -> Result<Response<ProblemId>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;
        let (user_id, perm) = auth.ok_or_default()?;

        let uuid = Uuid::parse_str(&req.request_id).map_err(Error::InvaildUUID)?;
        if let Some(x) = self.dup.check_i32(user_id, &uuid) {
            return Ok(Response::new(x.into()));
        };

        if !(perm.can_root() || perm.can_manage_problem()) {
            return Err(Error::RequirePermission(Entity::DEBUG_NAME).into());
        }

        let mut model: ActiveModel = Default::default();
        model.user_id = ActiveValue::Set(user_id);

        fill_active_model!(
            model, req.info, title, difficulty, time, memory, tags, content, match_rule, order
        );

        let model = model.save(db).await.map_err(Into::<Error>::into)?;

        self.dup.store_i32(user_id, uuid, model.id.clone().unwrap());

        tracing::debug!(id = model.id.clone().unwrap(), "problem_created");

        Ok(Response::new(model.id.unwrap().into()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn update(&self, req: Request<UpdateProblemRequest>) -> Result<Response<()>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;

        let (user_id, _perm) = auth.ok_or_default()?;

        let uuid = Uuid::parse_str(&req.request_id).map_err(Error::InvaildUUID)?;
        if self.dup.check_i32(user_id, &uuid).is_some() {
            return Ok(Response::new(()));
        };

        tracing::trace!(id = req.id.id);

        let mut model = Entity::write_filter(Entity::find_by_id(req.id), &auth)?
            .one(db)
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB(Entity::DEBUG_NAME))?
            .into_active_model();

        fill_exist_active_model!(
            model,
            req.info,
            title,
            difficulty,
            time,
            memory,
            tags,
            content,
            match_rule,
            ac_rate,
            submit_count,
            order
        );

        let model = model.update(db).await.map_err(Into::<Error>::into)?;

        self.dup.store_i32(user_id, uuid, model.id);

        Ok(Response::new(()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn remove(&self, req: Request<ProblemId>) -> Result<Response<()>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;

        let result = Entity::write_filter(Entity::delete_by_id(Into::<i32>::into(req.id)), &auth)?
            .exec(db)
            .await
            .map_err(Into::<Error>::into)?;

        if result.rows_affected == 0 {
            return Err(Error::NotInDB(Entity::DEBUG_NAME).into());
        }

        tracing::debug!(id = req.id);

        Ok(Response::new(()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn add_to_contest(
        &self,
        req: Request<AddProblemToContestRequest>,
    ) -> Result<Response<()>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;
        let (user_id, perm) = auth.ok_or_default()?;

        let (contest, model) = try_join!(
            spawn(contest::Entity::read_by_id(req.contest_id.id, &auth)?.one(db)),
            spawn(Entity::read_by_id(req.problem_id.id, &auth)?.one(db))
        )
        .unwrap();

        let contest = contest
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB("contest"))?;
        let model = model
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB(Entity::DEBUG_NAME))?;

        if !(perm.can_root() || perm.can_link()) {
            if contest.hoster != user_id {
                return Err(Error::NotInDB("contest").into());
            }
            if model.user_id != user_id {
                return Err(Error::NotInDB(Entity::DEBUG_NAME).into());
            }
            if !perm.can_manage_contest() {
                return Err(Error::RequirePermission("Contest").into());
            }
        }

        let mut model = model.into_active_model();
        model.contest_id = ActiveValue::Set(Some(req.problem_id.id));
        model.save(db).await.map_err(Into::<Error>::into)?;

        Ok(Response::new(()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn remove_from_contest(
        &self,
        req: Request<AddProblemToContestRequest>,
    ) -> Result<Response<()>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;

        let (_, perm) = auth.ok_or_default()?;

        if !(perm.can_root() || perm.can_link()) {
            return Err(Error::RequirePermission("TODO").into());
        }

        let mut problem = Entity::write_by_id(req.problem_id, &auth)?
            .columns([Column::Id, Column::ContestId])
            .one(db)
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB(Entity::DEBUG_NAME))?
            .into_active_model();

        problem.contest_id = ActiveValue::Set(None);

        problem.save(db).await.map_err(Into::<Error>::into)?;

        Ok(Response::new(()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn publish(&self, req: Request<ProblemId>) -> Result<Response<()>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;
        let (_, perm) = auth.ok_or_default()?;

        tracing::debug!(id = req.id);

        let mut query = Entity::find_by_id(Into::<i32>::into(req));

        if !perm.can_publish() {
            query = Entity::write_filter(query, &auth)?;
        }

        let mut problem = query
            .columns([Column::Id, Column::ContestId])
            .one(db)
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB(Entity::DEBUG_NAME))?
            .into_active_model();

        problem.public = ActiveValue::Set(true);

        problem.save(db).await.map_err(Into::<Error>::into)?;

        Ok(Response::new(()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn unpublish(&self, req: Request<ProblemId>) -> Result<Response<()>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;
        let (_, perm) = auth.ok_or_default()?;

        tracing::debug!(id = req.id);

        let mut query = Entity::find_by_id(Into::<i32>::into(req));

        if !perm.can_publish() {
            query = Entity::write_filter(query, &auth)?;
        }

        let mut problem = query
            .columns([Column::Id, Column::ContestId])
            .one(db)
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB(Entity::DEBUG_NAME))?
            .into_active_model();

        problem.public = ActiveValue::Set(false);

        problem.save(db).await.map_err(Into::<Error>::into)?;

        Ok(Response::new(()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn full_info_by_contest(
        &self,
        req: Request<AddProblemToContestRequest>,
    ) -> Result<Response<ProblemFullInfo>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;

        let parent = contest::Entity::related_read_by_id(&auth, Into::<i32>::into(req.contest_id))
            .one(db)
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB("contest"))?;

        let model = parent
            .find_related(Entity)
            .filter(Column::Id.eq(Into::<i32>::into(req.problem_id)))
            .one(db)
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB(Entity::DEBUG_NAME))?;

        Ok(Response::new(model.into()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn list_by_contest(
        &self,
        req: Request<ListByRequest>,
    ) -> Result<Response<ListProblemResponse>, Status> {
        let (auth, req) = self.parse_request(req).await?;

        let mut reverse = false;
        let mut pager: Pager<Entity> = match req.request.ok_or(Error::NotInPayload("request"))? {
            list_by_request::Request::ParentId(ppk) => {
                tracing::debug!(id = ppk);
                Pager::parent_sorted_search(ppk, ProblemSortBy::Order, false)
            }
            list_by_request::Request::Pager(old) => {
                reverse = old.reverse;
                <Pager<Entity> as HasParentPager<contest::Entity, Entity>>::from_raw(
                    old.session,
                    self,
                )?
            }
        };

        let list = pager
            .fetch(req.size, req.offset.unwrap_or_default(), reverse, &auth)
            .await?
            .into_iter()
            .map(|x| x.into())
            .collect();

        let next_session = pager.into_raw(self);

        Ok(Response::new(ListProblemResponse { list, next_session }))
    }
}
