use super::tools::*;

use grpc::backend::testcase_server::*;

use crate::entity::{
    testcase::{Paginator, *},
    *,
};

impl<'a> From<WithAuth<'a, Model>> for TestcaseFullInfo {
    fn from(value: WithAuth<'a, Model>) -> Self {
        let model = value.1;
        let writable = Entity::writable(&model, value.0);
        TestcaseFullInfo {
            id: model.id,
            score: model.score,
            inputs: model.input,
            outputs: model.output,
            writable,
        }
    }
}

impl WithAuthTrait for Model {}

impl From<PartialModel> for TestcaseInfo {
    fn from(value: PartialModel) -> Self {
        TestcaseInfo {
            id: value.id,
            score: value.score,
        }
    }
}

#[async_trait]
impl Testcase for ArcServer {
    #[instrument(
        skip_all,
        level = "info",
        name = "endpoint.Testcase.list",
        err(level = "debug", Display)
    )]
    async fn list(
        &self,
        req: Request<ListTestcaseRequest>,
    ) -> Result<Response<ListTestcaseResponse>, Status> {
        let (auth, req) = self.rate_limit(req).in_current_span().await?;

        req.bound_check()?;

        let paginator = match req.request.ok_or(Error::NotInPayload("request"))? {
            list_testcase_request::Request::Create(create) => {
                let start_from_end = create.order == Order::Descend as i32;
                if let Some(parent) = create.problem_id {
                    Paginator::new_parent(parent, start_from_end)
                } else {
                    Paginator::new(start_from_end)
                }
            }
            list_testcase_request::Request::Paginator(x) => self.crypto.decode(x)?,
        };
        let mut paginator = paginator.with_auth(&auth).with_db(&self.db);

        let list = paginator
            .fetch(req.size, req.offset)
            .in_current_span()
            .await?;
        let remain = paginator.remain().in_current_span().await?;

        let paginator = paginator.into_inner();

        Ok(Response::new(ListTestcaseResponse {
            list: list.into_iter().map(Into::into).collect(),
            paginator: self.crypto.encode(paginator)?,
            remain,
        }))
    }
    #[instrument(
        skip_all,
        level = "info",
        name = "endpoint.Testcase.create",
        err(level = "debug", Display)
    )]
    async fn create(&self, req: Request<CreateTestcaseRequest>) -> Result<Response<Id>, Status> {
        let (auth, req) = self.rate_limit(req).in_current_span().await?;
        req.bound_check()?;

        let (user_id, perm) = auth.assume_login()?;
        perm.super_user()?;

        req.get_or_insert(|req| async move {
            let mut model: ActiveModel = Default::default();
            model.user_id = ActiveValue::Set(user_id);

            fill_active_model!(model, req.info, input, output, score);

            let model = model
                .save(self.db.deref())
                .instrument(info_span!("save").or_current())
                .await
                .map_err(Into::<Error>::into)?;

            let id = *model.id.as_ref();

            tracing::info!(count.testcase = 1, id = id);

            Ok(id.into())
        })
        .await
        .with_grpc()
        .into()
    }
    #[instrument(
        skip_all,
        level = "info",
        name = "endpoint.Testcase.update",
        err(level = "debug", Display)
    )]
    async fn update(&self, req: Request<UpdateTestcaseRequest>) -> Result<Response<()>, Status> {
        let (auth, req) = self.rate_limit(req).in_current_span().await?;
        let (_, perm) = auth.assume_login()?;
        req.bound_check()?;

        req.get_or_insert(|req| async move {
            tracing::trace!(id = req.id);
            let mut model = Entity::write_filter(Entity::find_by_id(req.id), &auth)?
                .one(self.db.deref())
                .instrument(info_span!("fetch").or_current())
                .await
                .map_err(Into::<Error>::into)?
                .ok_or(Error::NotInDB)?
                .into_active_model();

            fill_exist_active_model!(model, req.info, input, output, score);

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
        name = "endpoint.Testcase.remove",
        err(level = "debug", Display)
    )]
    async fn remove(&self, req: Request<RemoveRequest>) -> Result<Response<()>, Status> {
        let (auth, req) = self.rate_limit(req).in_current_span().await?;

        req.get_or_insert(|req| async move {
            let result =
                Entity::write_filter(Entity::delete_by_id(Into::<i32>::into(req.id)), &auth)?
                    .exec(self.db.deref())
                    .instrument(info_span!("remove").or_current())
                    .await
                    .map_err(Into::<Error>::into)?;

            if result.rows_affected == 0 {
                Err(Error::NotInDB)
            } else {
                tracing::info!(counter.testcase = -1, id = req.id);
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
        name = "endpoint.Testcase.add_to_problem",
        err(level = "debug", Display)
    )]
    async fn add_to_problem(
        &self,
        req: Request<AddTestcaseToProblemRequest>,
    ) -> Result<Response<()>, Status> {
        let (auth, req) = self.rate_limit(req).in_current_span().await?;

        req.get_or_insert(|req| async move {
            let (problem, model) = tokio::try_join!(
                problem::Entity::write_by_id(req.problem_id, &auth)?
                    .one(self.db.deref())
                    .instrument(debug_span!("find_parent").or_current()),
                Entity::write_by_id(req.testcase_id, &auth)?
                    .one(self.db.deref())
                    .instrument(debug_span!("find_child").or_current())
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
        name = "endpoint.Testcase.remove_from_problem",
        err(level = "debug", Display)
    )]
    async fn remove_from_problem(
        &self,
        req: Request<AddTestcaseToProblemRequest>,
    ) -> Result<Response<()>, Status> {
        let (auth, req) = self.rate_limit(req).in_current_span().await?;

        req.get_or_insert(|req| async move {
            let (problem, model) = tokio::try_join!(
                problem::Entity::write_by_id(req.problem_id, &auth)?
                    .one(self.db.deref())
                    .instrument(debug_span!("find_parent").or_current()),
                Entity::write_by_id(req.testcase_id, &auth)?
                    .one(self.db.deref())
                    .instrument(debug_span!("find_child").or_current())
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
        level = "info",
        name = "endpoint.Testcase.full_info_by_problem",
        err(level = "debug", Display)
    )]
    async fn full_info_by_problem(
        &self,
        req: Request<ListTestcaseByProblemRequest>,
    ) -> Result<Response<TestcaseFullInfo>, Status> {
        let (auth, req) = self.rate_limit(req).in_current_span().await?;
        auth.perm().admin()?;

        let parent: problem::IdModel =
            problem::Entity::related_read_by_id(&auth, Into::<i32>::into(req.problem_id), &self.db)
                .in_current_span()
                .await?;

        let model = parent
            .upgrade()
            .find_related(Entity)
            .filter(Column::Id.eq(Into::<i32>::into(req.testcase_id)))
            .one(self.db.deref())
            .instrument(info_span!("fetch").or_current())
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB)?;

        Ok(Response::new(model.with_auth(&auth).into()))
    }
}
