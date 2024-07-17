use super::tools::*;

use grpc::backend::testcase_server::*;

use crate::entity::{testcase::*, *};

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

impl From<Model> for TestcaseInfo {
    fn from(value: Model) -> Self {
        TestcaseInfo {
            id: value.id,
            score: value.score,
        }
    }
}

#[async_trait]
impl Testcase for ArcServer {
    #[instrument(skip_all, level = "debug")]
    async fn list(
        &self,
        req: Request<ListTestcaseRequest>,
    ) -> Result<Response<ListTestcaseResponse>, Status> {
        todo!()
    }
    #[instrument(skip_all, level = "debug")]
    async fn create(&self, req: Request<CreateTestcaseRequest>) -> Result<Response<Id>, Status> {
        let (auth, req) = self.rate_limit(req).in_current_span().await?;
        let (user_id, perm) = auth.auth_or_guest()?;

        req.bound_check()?;

        let uuid = Uuid::parse_str(&req.request_id).map_err(Error::InvaildUUID)?;
        if let Some(x) = self.dup.check::<Id>(user_id, uuid) {
            return Ok(Response::new(x));
        };

        if !(perm.super_user()) {
            return Err(Error::RequirePermission(RoleLv::Super).into());
        }

        let mut model: ActiveModel = Default::default();
        model.user_id = ActiveValue::Set(user_id);

        fill_active_model!(model, req.info, input, output, score);

        let model = model
            .save(self.db.deref())
            .instrument(info_span!("save").or_current())
            .await
            .map_err(Into::<Error>::into)?;

        let id: Id = model.id.clone().unwrap().into();

        self.dup.store(user_id, uuid, id.clone());

        tracing::debug!(id = id.id, "tetscase_created");

        Ok(Response::new(id))
    }
    #[instrument(skip_all, level = "debug")]
    async fn update(&self, req: Request<UpdateTestcaseRequest>) -> Result<Response<()>, Status> {
        let (auth, req) = self.rate_limit(req).in_current_span().await?;
        let (user_id, _perm) = auth.auth_or_guest()?;

        req.bound_check()?;

        let uuid = Uuid::parse_str(&req.request_id).map_err(Error::InvaildUUID)?;
        if let Some(x) = self.dup.check::<()>(user_id, uuid) {
            return Ok(Response::new(x));
        };

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
            .await
            .map_err(atomic_fail)?;

        self.dup.store(user_id, uuid, ());

        Ok(Response::new(()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn remove(&self, req: Request<Id>) -> Result<Response<()>, Status> {
        let (auth, req) = self.rate_limit(req).in_current_span().await?;

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
    async fn add_to_problem(
        &self,
        req: Request<AddTestcaseToProblemRequest>,
    ) -> Result<Response<()>, Status> {
        let (auth, req) = self.rate_limit(req).in_current_span().await?;
        let (user_id, perm) = auth.auth_or_guest()?;

        if !perm.super_user() {
            return Err(Error::RequirePermission(RoleLv::Super).into());
        }

        let (problem, model) = tokio::try_join!(
            problem::Entity::read_by_id(req.problem_id, &auth)?
                .one(self.db.deref())
                .instrument(debug_span!("find_parent").or_current()),
            Entity::read_by_id(req.testcase_id, &auth)?
                .one(self.db.deref())
                .instrument(debug_span!("find_child").or_current())
        )
        .map_err(Into::<Error>::into)?;

        let problem = problem.ok_or(Error::NotInDB)?;
        let model = model.ok_or(Error::NotInDB)?;

        if !(perm.admin()) && problem.user_id != user_id {
            return Err(Error::UnownedAdd("problem").into());
        }

        let mut model = model.into_active_model();
        model.problem_id = ActiveValue::Set(Some(req.problem_id));
        model
            .save(self.db.deref())
            .instrument(info_span!("update_child").or_current())
            .await
            .map_err(Into::<Error>::into)?;

        Ok(Response::new(()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn remove_from_problem(
        &self,
        req: Request<AddTestcaseToProblemRequest>,
    ) -> Result<Response<()>, Status> {
        let (auth, req) = self.rate_limit(req).in_current_span().await?;

        let mut test = Entity::write_by_id(req.problem_id, &auth)?
            .columns([Column::Id, Column::ProblemId])
            .one(self.db.deref())
            .instrument(info_span!("fetch").or_current())
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB)?
            .into_active_model();

        test.problem_id = ActiveValue::Set(None);

        test.update(self.db.deref())
            .instrument(info_span!("update").or_current())
            .await
            .map_err(atomic_fail)?;

        Ok(Response::new(()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn full_info_by_problem(
        &self,
        req: Request<AddTestcaseToProblemRequest>,
    ) -> Result<Response<TestcaseFullInfo>, Status> {
        let (auth, req) = self.rate_limit(req).in_current_span().await?;

        tracing::debug!(problem_id = req.problem_id, testcase_id = req.testcase_id);

        let (_, perm) = auth.auth_or_guest()?;

        if !perm.admin() {
            return Err(Error::RequirePermission(RoleLv::Root).into());
        }

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
