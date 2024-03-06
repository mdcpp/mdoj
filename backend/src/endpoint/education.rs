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
        let (auth, rev, size, offset, pager) = parse_pager_param!(self, req);

        let (pager, models) = match pager {
            list_education_request::Request::StartFromEnd(rev) => {
                Paginator::new_fetch((), &auth, size, offset, rev, &self.db)
                    .in_current_span()
                    .await
            }
            list_education_request::Request::Pager(old) => {
                let span = tracing::info_span!("paginate").or_current();
                let pager: Paginator = span.in_scope(|| self.crypto.decode(old.session))?;
                pager
                    .fetch(&auth, size, offset, rev, &self.db)
                    .instrument(span)
                    .await
            }
        }?;

        let remain = pager.remain(&auth, &self.db).in_current_span().await?;

        let next_session = self.crypto.encode(pager)?;
        let list = models.into_iter().map(|x| x.into()).collect();

        Ok(Response::new(ListEducationResponse {
            list,
            next_session,
            remain,
        }))
    }
    #[instrument(skip_all, level = "debug")]
    async fn create(
        &self,
        req: Request<CreateEducationRequest>,
    ) -> Result<Response<EducationId>, Status> {
        let (auth, req) = self
            .parse_request_n(req, NonZeroU32!(5))
            .in_current_span()
            .await?;
        let (user_id, perm) = auth.ok_or_default()?;

        check_length!(SHORT_ART_SIZE, req.info, title);
        check_length!(LONG_ART_SIZE, req.info, content);

        let uuid = Uuid::parse_str(&req.request_id).map_err(Error::InvaildUUID)?;
        if let Some(x) = self.dup.check::<EducationId>(user_id, uuid) {
            return Ok(Response::new(x));
        };

        if !(perm.super_user()) {
            return Err(Error::RequirePermission(RoleLv::Super).into());
        }

        let mut model: ActiveModel = Default::default();
        model.user_id = ActiveValue::Set(user_id);

        fill_active_model!(model, req.info, title, content);

        let model = model
            .save(self.db.deref())
            .instrument(info_span!("save").or_current())
            .await
            .map_err(Into::<Error>::into)?;

        let id: EducationId = model.id.clone().unwrap().into();
        self.dup.store(user_id, uuid, id.clone());

        tracing::debug!(id = id.id, "education_created");
        self.metrics.education(1);

        Ok(Response::new(id))
    }
    #[instrument(skip_all, level = "debug")]
    async fn update(&self, req: Request<UpdateEducationRequest>) -> Result<Response<()>, Status> {
        let (auth, req) = self
            .parse_request_n(req, NonZeroU32!(5))
            .in_current_span()
            .await?;
        let (user_id, _perm) = auth.ok_or_default()?;

        check_exist_length!(SHORT_ART_SIZE, req.info, title);
        check_exist_length!(LONG_ART_SIZE, req.info, content);

        let uuid = Uuid::parse_str(&req.request_id).map_err(Error::InvaildUUID)?;
        if let Some(x) = self.dup.check::<()>(user_id, uuid) {
            return Ok(Response::new(x));
        };

        tracing::trace!(id = req.id.id);

        let mut model = Entity::write_filter(Entity::find_by_id(req.id), &auth)?
            .one(self.db.deref())
            .instrument(info_span!("fetch").or_current())
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB)?
            .into_active_model();

        fill_exist_active_model!(model, req.info, title, content);

        model
            .update(self.db.deref())
            .instrument(info_span!("update").or_current())
            .await
            .map_err(atomic_fail)?;

        self.dup.store(user_id, uuid, ());

        Ok(Response::new(()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn remove(&self, req: Request<EducationId>) -> Result<Response<()>, Status> {
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
        self.metrics.education(-1);

        Ok(Response::new(()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn add_to_problem(
        &self,
        req: Request<AddEducationToProblemRequest>,
    ) -> Result<Response<()>, Status> {
        let (auth, req) = self
            .parse_request_n(req, NonZeroU32!(5))
            .in_current_span()
            .await?;
        let (user_id, perm) = auth.ok_or_default()?;

        let (problem, model) = tokio::try_join!(
            problem::Entity::read_by_id(req.problem_id.id, &auth)?
                .one(self.db.deref())
                .instrument(info_span!("fetch_parent").or_current()),
            Entity::read_by_id(req.education_id.id, &auth)?
                .one(self.db.deref())
                .instrument(info_span!("fetch_child").or_current())
        )
        .map_err(Into::<Error>::into)?;

        let problem = problem.ok_or(Error::NotInDB)?;
        let model = model.ok_or(Error::NotInDB)?;

        if !(perm.super_user()) {
            if problem.user_id != user_id {
                return Err(Error::NotInDB.into());
            }
            if model.user_id != user_id {
                return Err(Error::NotInDB.into());
            }
        }

        let mut model = model.into_active_model();
        model.problem_id = ActiveValue::Set(Some(req.problem_id.id));
        model
            .update(self.db.deref())
            .instrument(info_span!("update").or_current())
            .await
            .map_err(atomic_fail)?;

        Ok(Response::new(()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn remove_from_problem(
        &self,
        req: Request<AddEducationToProblemRequest>,
    ) -> Result<Response<()>, Status> {
        let (auth, req) = self
            .parse_request_n(req, NonZeroU32!(5))
            .in_current_span()
            .await?;

        let mut model = Entity::write_by_id(req.problem_id.id, &auth)?
            .columns([Column::Id, Column::ProblemId])
            .one(self.db.deref())
            .instrument(info_span!("fetch").or_current())
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB)?
            .into_active_model();

        model.problem_id = ActiveValue::Set(None);

        model
            .save(self.db.deref())
            .instrument(info_span!("remove").or_current())
            .await
            .map_err(Into::<Error>::into)?;

        Ok(Response::new(()))
    }

    #[instrument(skip_all, level = "debug")]
    async fn list_by_problem(
        &self,
        req: Request<ListByRequest>,
    ) -> Result<Response<ListEducationResponse>, Status> {
        let (auth, rev, size, offset, pager) = parse_pager_param!(self, req);

        let (pager, models) = match pager {
            list_by_request::Request::Create(create) => {
                ParentPaginator::new_fetch(
                    create.parent_id,
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

        Ok(Response::new(ListEducationResponse {
            list,
            next_session,
            remain,
        }))
    }
    #[instrument(skip_all, level = "debug")]
    async fn full_info_by_problem(
        &self,
        req: Request<AddEducationToProblemRequest>,
    ) -> Result<Response<EducationFullInfo>, Status> {
        let (auth, req) = self
            .parse_request_n(req, NonZeroU32!(5))
            .in_current_span()
            .await?;

        let parent: problem::IdModel =
            problem::Entity::related_read_by_id(&auth, Into::<i32>::into(req.problem_id), &self.db)
                .in_current_span()
                .await?;
        let model = parent
            .upgrade()
            .find_related(Entity)
            .filter(Column::Id.eq(Into::<i32>::into(req.education_id)))
            .one(self.db.deref())
            .instrument(info_span!("fetch").or_current())
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB)?;

        Ok(Response::new(model.into()))
    }
}
