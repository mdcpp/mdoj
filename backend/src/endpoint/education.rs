use super::*;

use grpc::backend::education_server::*;

use crate::entity::{education::Paginator, education::*, problem};

impl<'a> From<WithAuth<'a, Model>> for EducationFullInfo {
    fn from(value: WithAuth<'a, Model>) -> Self {
        let model = value.1;
        let writable = Entity::writable(&model, value.0);
        EducationFullInfo {
            info: EducationInfo {
                id: model.id,
                title: model.title,
            },
            content: model.content,
            problem: model.problem_id.map(Into::into),
            writable,
        }
    }
}

impl WithAuthTrait for Model {}

impl From<PartialModel> for EducationInfo {
    fn from(value: PartialModel) -> Self {
        EducationInfo {
            id: value.id,
            title: value.title,
        }
    }
}

#[async_trait]
impl Education for ArcServer {
    #[instrument(
        skip_all,
        level = "info",
        name = "oj.backend.Education/list",
        err(level = "debug", Display)
    )]
    async fn list(
        &self,
        req: Request<ListEducationRequest>,
    ) -> Result<Response<ListEducationResponse>, Status> {
        let (auth, req) = self.rate_limit(req).in_current_span().await?;

        req.bound_check()?;

        let paginator = match req.request.ok_or(Error::NotInPayload("request"))? {
            list_education_request::Request::Create(create) => {
                let query = create.query.unwrap_or_default();
                let start_from_end = create.order == Order::Descend as i32;
                if let Some(text) = query.text {
                    Paginator::new_text(text, start_from_end)
                } else if let Some(parent) = query.problem_id {
                    Paginator::new_parent(parent, start_from_end)
                } else {
                    Paginator::new(start_from_end)
                }
            }
            list_education_request::Request::Paginator(x) => self.crypto.decode(x)?,
        };
        let mut paginator = paginator.with_auth(&auth).with_db(&self.db);

        let list = paginator
            .fetch(req.size, req.offset)
            .in_current_span()
            .await?;
        let remain = paginator.remain().in_current_span().await?;

        let paginator = paginator.into_inner();

        Ok(Response::new(ListEducationResponse {
            list: list.into_iter().map(Into::into).collect(),
            paginator: self.crypto.encode(paginator)?,
            remain,
        }))
    }
    #[instrument(
        skip_all,
        level = "info",
        name = "oj.backend.Education/create",
        err(level = "debug", Display)
    )]
    async fn create(&self, req: Request<CreateEducationRequest>) -> Result<Response<Id>, Status> {
        let (auth, req) = self.rate_limit(req).in_current_span().await?;
        req.bound_check()?;

        let (user_id, perm) = auth.assume_login()?;
        perm.super_user()?;

        req.get_or_insert(|req| async {
            let mut model: ActiveModel = Default::default();
            model.user_id = ActiveValue::Set(user_id);

            fill_active_model!(model, req.info, title, content);

            let model = model
                .save(self.db.deref())
                .instrument(info_span!("save").or_current())
                .await
                .map_err(Into::<Error>::into)?;

            let id = *model.id.as_ref();

            info!(count.education.count = 1, id = id);

            Ok(id.into())
        })
        .await
        .with_grpc()
        .into()
    }
    #[instrument(
        skip_all,
        level = "info",
        name = "oj.backend.Education/update",
        err(level = "debug", Display)
    )]
    async fn update(&self, req: Request<UpdateEducationRequest>) -> Result<Response<()>, Status> {
        let (auth, req) = self.rate_limit(req).in_current_span().await?;
        req.bound_check()?;

        req.get_or_insert(|req| async move {
            trace!(id = req.id);
            let mut model = Entity::find_by_id(req.id)
                .with_auth(&auth)
                .write()?
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
                .await?;
            Ok(())
        })
        .await
        .with_grpc()
        .into()
    }
    #[instrument(
        skip_all,
        level = "info",
        name = "oj.backend.Education/remove",
        err(level = "debug", Display)
    )]
    async fn remove(&self, req: Request<RemoveRequest>) -> Result<Response<()>, Status> {
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
                info!(counter.education = -1, id = req.id);
                Ok(())
            }
        })
        .await
        .with_grpc()
        .into()
    }
    #[instrument(
        skip_all,
        level = "info",
        name = "oj.backend.Education/add_to_problem",
        err(level = "debug", Display)
    )]
    async fn add_to_problem(
        &self,
        req: Request<AddEducationToProblemRequest>,
    ) -> Result<Response<()>, Status> {
        let (auth, req) = self.rate_limit(req).in_current_span().await?;
        auth.perm().super_user()?;

        req.get_or_insert(|req| async move {
            let (problem, model) = tokio::try_join!(
                problem::Entity::write_by_id(req.problem_id, &auth)?
                    .one(self.db.deref())
                    .instrument(info_span!("fetch_parent").or_current()),
                Entity::write_by_id(req.education_id, &auth)?
                    .one(self.db.deref())
                    .instrument(info_span!("fetch_child").or_current())
            )
            .map_err(Into::<Error>::into)?;

            problem.ok_or(Error::NotInDB)?;
            let mut model = model.ok_or(Error::NotInDB)?.into_active_model();

            model.problem_id = ActiveValue::Set(Some(req.problem_id));
            model
                .update(self.db.deref())
                .instrument(info_span!("update").or_current())
                .await?;
            Ok(())
        })
        .await
        .with_grpc()
        .into()
    }
    #[instrument(
        skip_all,
        level = "info",
        name = "oj.backend.Education/remove_from_problem",
        err(level = "debug", Display)
    )]
    async fn remove_from_problem(
        &self,
        req: Request<AddEducationToProblemRequest>,
    ) -> Result<Response<()>, Status> {
        let (auth, req) = self.rate_limit(req).in_current_span().await?;
        auth.perm().super_user()?;

        req.get_or_insert(|req| async move {
            let (problem, model) = tokio::try_join!(
                problem::Entity::write_by_id(req.problem_id, &auth)?
                    .one(self.db.deref())
                    .instrument(info_span!("fetch_parent").or_current()),
                Entity::write_by_id(req.education_id, &auth)?
                    .one(self.db.deref())
                    .instrument(info_span!("fetch_child").or_current())
            )
            .map_err(Into::<Error>::into)?;

            problem.ok_or(Error::NotInDB)?;
            let mut model = model.ok_or(Error::NotInDB)?.into_active_model();

            model.problem_id = ActiveValue::Set(None);
            model
                .update(self.db.deref())
                .instrument(info_span!("update").or_current())
                .await?;
            Ok(())
        })
        .await
        .with_grpc()
        .into()
    }

    #[instrument(
        skip_all,
        level = "info",full_info_by_problem
        name = "oj.backend.Education/list",
        err(level = "debug", Display)
    )]
    async fn full_info_by_problem(
        &self,
        req: Request<ListEducationByProblemRequest>,
    ) -> Result<Response<EducationFullInfo>, Status> {
        let (auth, req) = self.rate_limit(req).in_current_span().await?;

        let parent: problem::IdModel =
            problem::Entity::related_read_by_id(&auth, req.problem_id, &self.db)
                .in_current_span()
                .await?;
        let model = parent
            .upgrade()
            .find_related(Entity)
            .filter(Column::Id.eq(req.education_id))
            .one(self.db.deref())
            .instrument(info_span!("fetch").or_current())
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB)?;

        Ok(Response::new(model.with_auth(&auth).into()))
    }
}
