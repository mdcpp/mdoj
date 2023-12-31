use super::endpoints::*;
use super::tools::*;

use crate::grpc::backend::education_set_server::*;
use crate::grpc::backend::*;

use entity::{education::*, *};

impl Filter for Entity {
    fn read_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
        if let Some(perm) = auth.user_perm() {
            if perm.can_root() || perm.can_manage_education() {
                return Ok(query);
            }
        }
        Err(Error::Unauthenticated)
    }
    fn write_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
        let (user_id, perm) = auth.ok_or_default()?;
        if perm.can_root() {
            return Ok(query);
        }
        if perm.can_manage_education() {
            return Ok(query.filter(education::Column::UserId.eq(user_id)));
        }
        Err(Error::Unauthenticated)
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
        Err(Error::PremissionDeny("Can't publish education"))
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
        Err(Error::PremissionDeny("Can't link education"))
    }
}

impl From<i32> for EducationId {
    fn from(value: i32) -> Self {
        Self { id: value }
    }
}

impl From<EducationId> for i32 {
    fn from(value: EducationId) -> Self {
        value.id
    }
}

impl From<Model> for EducationFullInfo {
    fn from(value: Model) -> Self {
        EducationFullInfo {
            info: value.clone().into(),
            content: value.content,
            problem: value.problem_id.map(Into::into),
        }
    }
}
impl From<Model> for EducationInfo {
    fn from(value: Model) -> Self {
        EducationInfo {
            id: value.id.into(),
            title: value.title,
        }
    }
}

#[async_trait]
impl EducationSet for Arc<Server> {
    #[instrument(skip_all, level = "debug")]
    async fn create(
        &self,
        req: Request<CreateEducationRequest>,
    ) -> Result<Response<EducationId>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;
        let (user_id, perm) = auth.ok_or_default()?;

        let uuid = Uuid::parse_str(&req.request_id).map_err(Error::InvaildUUID)?;
        if let Some(x) = self.dup.check_i32(user_id, &uuid) {
            return Ok(Response::new(x.into()));
        };

        if !(perm.can_root() || perm.can_manage_education()) {
            return Err(Error::PremissionDeny("Can't create education").into());
        }

        let mut model: ActiveModel = Default::default();
        model.user_id = ActiveValue::Set(user_id);

        fill_active_model!(model, req.info, title, content);

        let model = model.save(db).await.map_err(Into::<Error>::into)?;

        self.dup.store_i32(user_id, uuid, model.id.clone().unwrap());

        tracing::debug!(id = model.id.clone().unwrap());
        self.metrics.education.add(1, &[]);

        Ok(Response::new(model.id.unwrap().into()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn update(&self, req: Request<UpdateEducationRequest>) -> Result<Response<()>, Status> {
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
            .ok_or(Error::NotInDB("problem"))?
            .into_active_model();

        fill_exist_active_model!(model, req.info, title, content);

        let model = model.update(db).await.map_err(Into::<Error>::into)?;

        self.dup.store_i32(user_id, uuid, model.id);

        Ok(Response::new(()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn remove(&self, req: Request<EducationId>) -> Result<Response<()>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;

        Entity::write_filter(Entity::delete_by_id(Into::<i32>::into(req.id)), &auth)?
            .exec(db)
            .await
            .map_err(Into::<Error>::into)?;

        tracing::debug!(id = req.id);
        self.metrics.education.add(-1, &[]);

        Ok(Response::new(()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn link(&self, req: Request<EducationLink>) -> Result<Response<()>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;

        let (_, perm) = auth.ok_or_default()?;

        if !(perm.can_root() || perm.can_link()) {
            return Err(Error::PremissionDeny("Can't link problem").into());
        }

        let mut model = Entity::link_filter(Entity::find_by_id(req.problem_id.id), &auth)?
            .columns([Column::Id, Column::ProblemId])
            .one(db)
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB("problem"))?
            .into_active_model();

        model.problem_id = ActiveValue::Set(Some(req.problem_id.id));

        model.save(db).await.map_err(Into::<Error>::into)?;

        Ok(Response::new(()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn unlink(&self, req: Request<EducationLink>) -> Result<Response<()>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;

        let (_, perm) = auth.ok_or_default()?;

        if !(perm.can_root() || perm.can_link()) {
            return Err(Error::PremissionDeny("Can't link problem").into());
        }

        let mut model = Entity::link_filter(Entity::find_by_id(req.problem_id.id), &auth)?
            .columns([Column::Id, Column::ProblemId])
            .one(db)
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB("problem"))?
            .into_active_model();

        model.problem_id = ActiveValue::Set(None);

        model.save(db).await.map_err(Into::<Error>::into)?;

        Ok(Response::new(()))
    }

    #[instrument(skip_all, level = "debug")]
    async fn list_by_problem(
        &self,
        req: Request<ListByRequest>,
    ) -> Result<Response<ListEducationResponse>, Status> {
        let (auth, req) = self.parse_request(req).await?;

        let mut reverse = false;
        let mut pager: Pager<Entity> = match req.request.ok_or(Error::NotInPayload("request"))? {
            list_by_request::Request::ParentId(ppk) => {
                tracing::debug!(id = ppk);
                Pager::parent_search(ppk)
            }
            list_by_request::Request::Pager(old) => {
                reverse = old.reverse;
                <Pager<Entity> as HasParentPager<problem::Entity, Entity>>::from_raw(
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

        Ok(Response::new(ListEducationResponse { list, next_session }))
    }
    #[instrument(skip_all, level = "debug")]
    async fn full_info_by_problem(
        &self,
        req: Request<EducationLink>,
    ) -> Result<Response<EducationFullInfo>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;

        let parent = auth
            .get_user(db)
            .await?
            .find_related(problem::Entity)
            .columns([contest::Column::Id])
            .one(db)
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB("problem"))?;

        let model = parent
            .find_related(Entity)
            .filter(Column::Id.eq(Into::<i32>::into(req.problem_id)))
            .one(db)
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB("education"))?;

        Ok(Response::new(model.into()))
    }
}
