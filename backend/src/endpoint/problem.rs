use super::endpoints::*;
use super::tools::*;

use crate::grpc::backend::problem_set_server::*;
use crate::grpc::backend::*;

use entity::{problem::*, *};

#[async_trait]
impl Filter for Entity {
    fn read_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
        if let Some(perm) = auth.user_perm() {
            if perm.can_link() || perm.can_root() || perm.can_manage_problem() {
                return Ok(query);
            }
        }
        Ok(query.filter(Column::Public.eq(true)))
    }
    fn write_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
        if let Some(perm) = auth.user_perm() {
            if perm.can_root() {
                return Ok(query);
            }
            if perm.can_manage_problem() {
                let user_id = auth.user_id().unwrap();
                return Ok(query.filter(Column::UserId.eq(user_id)));
            }
        }
        Err(Error::PremissionDeny("Can't write problem"))
    }
}

#[async_trait]
impl ParentalFilter for Entity {
    fn publish_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
        if let Some(perm) = auth.user_perm() {
            if perm.can_root() {
                return Ok(query);
            }
            if perm.can_publish() {
                let user_id = auth.user_id().unwrap();
                return Ok(query.filter(Column::UserId.eq(user_id)));
            }
        }
        Err(Error::PremissionDeny("Can't publish problem"))
    }

    fn link_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
        if let Some(perm) = auth.user_perm() {
            if perm.can_root() {
                return Ok(query);
            }
            if perm.can_link() {
                let user_id = auth.user_id().unwrap();
                return Ok(query.filter(Column::UserId.eq(user_id)));
            }
        }
        Err(Error::PremissionDeny("Can't link problem"))
    }
}

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
            time: value.time,
            memory: value.memory,
            info: value.into(),
        }
    }
}

#[async_trait]
impl ProblemSet for Arc<Server> {
    async fn list(
        &self,
        req: Request<ListRequest>,
    ) -> Result<Response<ListProblemResponse>, Status> {
        let (auth, req) = self.parse_request(req).await?;

        let mut reverse = false;
        let mut pager: Pager<Entity> = match req.request.ok_or(Error::NotInPayload("request"))? {
            list_request::Request::Create(create) => {
                Pager::sort_search(create.sort_by(), create.reverse)
            }
            list_request::Request::Pager(old) => {
                reverse = old.reverse;
                <Pager<Entity> as HasParentPager<contest::Entity, Entity>>::from_raw(old.session)?
            }
        };

        let list = pager
            .fetch(req.size, req.offset.unwrap_or_default(), reverse, &auth)
            .await?
            .into_iter()
            .map(|x| x.into())
            .collect();

        let next_session = pager.into_raw();

        Ok(Response::new(ListProblemResponse {
            list,
            next_session,
        }))
    }
    async fn search_by_text(
        &self,
        req: Request<TextSearchRequest>,
    ) -> Result<Response<ListProblemResponse>, Status> {
        let (auth, req) = self.parse_request(req).await?;

        let mut reverse = false;
        let mut pager: Pager<Entity> = match req.request.ok_or(Error::NotInPayload("request"))? {
            text_search_request::Request::Text(create) => Pager::text_search(create),
            text_search_request::Request::Pager(old) => {
                reverse = old.reverse;
                <Pager<_> as HasParentPager<contest::Entity, Entity>>::from_raw(old.session)?
            }
        };

        let list = pager
            .fetch(req.size, req.offset.unwrap_or_default(), reverse, &auth)
            .await?
            .into_iter()
            .map(|x| x.into())
            .collect();

        let next_session = pager.into_raw();

        Ok(Response::new(ListProblemResponse {
            list,
            next_session,
        }))
    }
    async fn full_info(
        &self,
        req: Request<ProblemId>,
    ) -> Result<Response<ProblemFullInfo>, Status> {
        let db = DB.get().unwrap();
        let (_, req) = self.parse_request(req).await?;

        let query = Entity::find_by_id::<i32>(req.into()).filter(Column::Public.eq(true));
        let model = query
            .one(db)
            .await
            .map_err(|x| Into::<Error>::into(x))?
            .ok_or(Error::NotInDB("problem"))?;

        Ok(Response::new(model.into()))
    }
    async fn create(
        &self,
        req: Request<CreateProblemRequest>,
    ) -> Result<Response<ProblemId>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;
        let (user_id, perm) = auth.ok_or_default()?;

        let uuid = Uuid::parse_str(&req.request_id).map_err(|e| Error::InvaildUUID(e))?;
        if let Some(x) = self.dup.check(user_id, &uuid) {
            return Ok(Response::new(x.into()));
        };

        if !(perm.can_root() || perm.can_manage_problem()) {
            return Err(Error::PremissionDeny("Can't create problem").into());
        }

        let mut model: ActiveModel = Default::default();
        model.user_id = ActiveValue::Set(user_id);

        fill_active_model!(
            model, req.info, title, difficulty, time, memory, tags, content, match_rule
        );

        let model = model.save(db).await.map_err(|x| Into::<Error>::into(x))?;

        self.dup.store(user_id, uuid, model.id.clone().unwrap());

        Ok(Response::new(model.id.unwrap().into()))
    }
    async fn update(&self, req: Request<UpdateProblemRequest>) -> Result<Response<()>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;

        let (user_id, perm) = auth.ok_or_default()?;

        let uuid = Uuid::parse_str(&req.request_id).map_err(|e| Error::InvaildUUID(e))?;
        if let Some(_) = self.dup.check(user_id, &uuid) {
            return Ok(Response::new(()));
        };

        if !(perm.can_root() || perm.can_manage_problem()) {
            return Err(Error::PremissionDeny("Can't update problem").into());
        }

        let mut model = Entity::write_filter(Entity::find_by_id(req.id), &auth)?
            .one(db)
            .await
            .map_err(|x| Into::<Error>::into(x))?
            .ok_or(Error::NotInDB("problem"))?
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
            submit_count
        );

        let model = model.update(db).await.map_err(|x| Into::<Error>::into(x))?;

        self.dup.store(user_id, uuid, model.id);

        Ok(Response::new(()))
    }
    async fn remove(&self, req: Request<ProblemId>) -> Result<Response<()>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;

        Entity::write_filter(Entity::delete_by_id(Into::<i32>::into(req.id)), &auth)?
            .exec(db)
            .await
            .map_err(|x| Into::<Error>::into(x))?;

        Ok(Response::new(()))
    }
    async fn link(&self, req: Request<ProblemLink>) -> Result<Response<()>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;

        let (_, perm) = auth.ok_or_default()?;

        if !(perm.can_root() || perm.can_link()) {
            return Err(Error::PremissionDeny("Can't link problem").into());
        }

        let mut problem = Entity::link_filter(Entity::find_by_id(req.problem_id), &auth)?
            .columns([Column::Id, Column::ContestId])
            .one(db)
            .await
            .map_err(|x| Into::<Error>::into(x))?
            .ok_or(Error::NotInDB("problem"))?
            .into_active_model();

        problem.contest_id = ActiveValue::Set(Some(req.contest_id.id));

        problem.save(db).await.map_err(|x| Into::<Error>::into(x))?;

        Ok(Response::new(()))
    }
    async fn unlink(&self, req: Request<ProblemLink>) -> Result<Response<()>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;

        let (_, perm) = auth.ok_or_default()?;

        if !(perm.can_root() || perm.can_link()) {
            return Err(Error::PremissionDeny("Can't link problem").into());
        }

        let mut problem = Entity::link_filter(Entity::find_by_id(req.problem_id), &auth)?
            .columns([Column::Id, Column::ContestId])
            .one(db)
            .await
            .map_err(|x| Into::<Error>::into(x))?
            .ok_or(Error::NotInDB("problem"))?
            .into_active_model();

        problem.contest_id = ActiveValue::Set(None);

        problem.save(db).await.map_err(|x| Into::<Error>::into(x))?;

        Ok(Response::new(()))
    }
    async fn publish(&self, req: Request<ProblemId>) -> Result<Response<()>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;

        let (_, perm) = auth.ok_or_default()?;

        let mut problem =
            Entity::publish_filter(Entity::find_by_id(Into::<i32>::into(req)), &auth)?
                .columns([Column::Id, Column::ContestId])
                .one(db)
                .await
                .map_err(|x| Into::<Error>::into(x))?
                .ok_or(Error::NotInDB("problem"))?
                .into_active_model();

        problem.public = ActiveValue::Set(true);

        problem.save(db).await.map_err(|x| Into::<Error>::into(x))?;

        Ok(Response::new(()))
    }
    async fn unpublish(&self, req: Request<ProblemId>) -> Result<Response<()>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;

        let (_, perm) = auth.ok_or_default()?;

        let mut problem =
            Entity::publish_filter(Entity::find_by_id(Into::<i32>::into(req)), &auth)?
                .columns([Column::Id, Column::ContestId])
                .one(db)
                .await
                .map_err(|x| Into::<Error>::into(x))?
                .ok_or(Error::NotInDB("problem"))?
                .into_active_model();

        problem.public = ActiveValue::Set(false);

        problem.save(db).await.map_err(|x| Into::<Error>::into(x))?;

        Ok(Response::new(()))
    }
    async fn full_info_by_contest(
        &self,
        req: Request<ProblemLink>,
    ) -> Result<Response<ProblemFullInfo>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;

        let parent = auth
            .get_user(db)
            .await?
            .find_related(contest::Entity)
            .columns([contest::Column::Id])
            .one(db)
            .await
            .map_err(|x| Into::<Error>::into(x))?
            .ok_or(Error::NotInDB("contest"))?;

        let model = parent
            .find_related(Entity)
            .filter(Column::Id.eq(Into::<i32>::into(req.problem_id)))
            .one(db)
            .await
            .map_err(|x| Into::<Error>::into(x))?
            .ok_or(Error::NotInDB("problem"))?;

        Ok(Response::new(model.into()))
    }
    async fn list_by_contest(
        &self,
        req: Request<ListByRequest>,
    ) -> Result<Response<ListProblemResponse>, Status> {
        let (auth, req) = self.parse_request(req).await?;

        let mut reverse = false;
        let mut pager: Pager<Entity> = match req.request.ok_or(Error::NotInPayload("request"))? {
            list_by_request::Request::ParentId(ppk) => Pager::parent_search(ppk),
            list_by_request::Request::Pager(old) => {
                reverse = old.reverse;
                <Pager<Entity> as HasParentPager<contest::Entity, Entity>>::from_raw(old.session)?
            }
        };

        let list = pager
            .fetch(req.size, req.offset.unwrap_or_default(), reverse, &auth)
            .await?
            .into_iter()
            .map(|x| x.into())
            .collect();

        let next_session = pager.into_raw();

        Ok(Response::new(ListProblemResponse {
            list,
            next_session,
        }))
    }
}
