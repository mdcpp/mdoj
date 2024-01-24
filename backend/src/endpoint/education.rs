use super::tools::*;

use crate::grpc::backend::education_set_server::*;
use crate::grpc::backend::*;

use crate::entity::{education::Paginator, education::*, *};

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
            info: EducationInfo {
                id: value.id.into(),
                title: value.title,
            },
            content: value.content,
            problem: value.problem_id.map(Into::into),
        }
    }
}
impl From<PartialModel> for EducationInfo {
    fn from(value: PartialModel) -> Self {
        EducationInfo {
            id: value.id.into(),
            title: value.title,
        }
    }
}

#[async_trait]
impl EducationSet for Arc<Server> {
    async fn list(
        &self,
        req: Request<ListEducationRequest>,
    ) -> Result<Response<ListEducationResponse>, Status> {
        let (auth, req) = self.parse_request(req).await?;
        let size = bound!(req.size, 64);
        let offset = bound!(req.offset(), 1024);
        let rev = req.reverse();

        let (pager, models) = match req.request.ok_or(Error::NotInPayload("request"))? {
            list_education_request::Request::Pager(pager) => {
                let pager: Paginator = self.crypto.decode(pager.session)?;
                pager.fetch(&auth, size, offset, rev).await
            }
            list_education_request::Request::StartFromEnd(rev) => {
                Paginator::new_fetch((), &auth, size, offset, rev).await
            }
        }?;

        let next_session = self.crypto.encode(pager)?;
        let list = models.into_iter().map(|x| x.into()).collect();

        Ok(Response::new(ListEducationResponse { list, next_session }))
    }
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

        if !(perm.super_user()) {
            return Err(Error::RequirePermission(RoleLv::Super).into());
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

        let result = Entity::write_filter(Entity::delete_by_id(Into::<i32>::into(req.id)), &auth)?
            .exec(db)
            .await
            .map_err(Into::<Error>::into)?;

        if result.rows_affected == 0 {
            return Err(Error::NotInDB(Entity::DEBUG_NAME).into());
        }

        tracing::debug!(id = req.id);
        self.metrics.education.add(-1, &[]);

        Ok(Response::new(()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn add_to_problem(
        &self,
        req: Request<AddEducationToProblemRequest>,
    ) -> Result<Response<()>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;
        let (user_id, perm) = auth.ok_or_default()?;

        let (problem, model) = try_join!(
            spawn(problem::Entity::read_by_id(req.problem_id.id, &auth)?.one(db)),
            spawn(Entity::read_by_id(req.education_id.id, &auth)?.one(db))
        )
        .unwrap();

        let problem = problem
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB("problem"))?;
        let model = model
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB(Entity::DEBUG_NAME))?;

        if !(perm.super_user()) {
            if problem.user_id != user_id {
                return Err(Error::NotInDB("problem").into());
            }
            if model.user_id != user_id {
                return Err(Error::NotInDB(Entity::DEBUG_NAME).into());
            }
        }

        let mut model = model.into_active_model();
        model.problem_id = ActiveValue::Set(Some(req.problem_id.id));
        model.save(db).await.map_err(Into::<Error>::into)?;

        Ok(Response::new(()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn remove_from_problem(
        &self,
        req: Request<AddEducationToProblemRequest>,
    ) -> Result<Response<()>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;

        let mut model = Entity::write_by_id(req.problem_id.id, &auth)?
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
        let size = bound!(req.size, 64);
        let offset = bound!(req.offset(), 1024);

        let (pager, models) = match req.request.ok_or(Error::NotInPayload("request"))? {
            list_by_request::Request::Create(create) => {
                tracing::debug!(id = create.parent_id);
                ParentPaginator::new_fetch(
                    create.parent_id,
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

        Ok(Response::new(ListEducationResponse { list, next_session }))
    }
    #[instrument(skip_all, level = "debug")]
    async fn full_info_by_problem(
        &self,
        req: Request<AddEducationToProblemRequest>,
    ) -> Result<Response<EducationFullInfo>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;

        let parent: problem::IdModel =
            problem::Entity::related_read_by_id(&auth, Into::<i32>::into(req.problem_id)).await?;
        let model = parent
            .upgrade()
            .find_related(Entity)
            .filter(Column::Id.eq(Into::<i32>::into(req.education_id)))
            .one(db)
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB(Entity::DEBUG_NAME))?;

        Ok(Response::new(model.into()))
    }
}
