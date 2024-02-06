use super::tools::*;

use crate::grpc::backend::testcase_set_server::*;
use crate::grpc::backend::*;

use crate::entity::{test::*, *};

impl From<i32> for TestcaseId {
    fn from(value: i32) -> Self {
        Self { id: value }
    }
}
impl From<TestcaseId> for i32 {
    fn from(value: TestcaseId) -> Self {
        value.id
    }
}

impl From<Model> for TestcaseFullInfo {
    fn from(value: Model) -> Self {
        TestcaseFullInfo {
            id: value.id.into(),
            score: value.score,
            inputs: value.input,
            outputs: value.output,
        }
    }
}

impl From<Model> for TestcaseInfo {
    fn from(value: Model) -> Self {
        TestcaseInfo {
            id: value.id.into(),
            score: value.score,
        }
    }
}

#[async_trait]
impl TestcaseSet for Arc<Server> {
    #[instrument(skip_all, level = "debug")]
    async fn list(
        &self,
        req: Request<ListTestcaseRequest>,
    ) -> Result<Response<ListTestcaseResponse>, Status> {
        let (auth, req) = self.parse_request(req).await?;

        let (rev, size) = split_rev(req.size);
        let size = bound!(size, 64);
        let offset = bound!(req.offset(), 1024);

        let (pager, models) = match req.request.ok_or(Error::NotInPayload("request"))? {
            list_testcase_request::Request::Pager(old) => {
                let pager: ColPaginator = self.crypto.decode(old.session)?;
                pager.fetch(&auth, size, offset, rev, &self.db).await
            }
            list_testcase_request::Request::StartFromEnd(rev) => {
                ColPaginator::new_fetch(Default::default(), &auth, size, offset, rev, &self.db)
                    .await
            }
        }?;

        let next_session = self.crypto.encode(pager)?;
        let list = models.into_iter().map(|x| x.into()).collect();

        Ok(Response::new(ListTestcaseResponse { list, next_session }))
    }
    #[instrument(skip_all, level = "debug")]
    async fn create(
        &self,
        req: Request<CreateTestcaseRequest>,
    ) -> Result<Response<TestcaseId>, Status> {
        let (auth, req) = self.parse_request(req).await?;
        let (user_id, perm) = auth.ok_or_default()?;

        check_length!(LONG_ART_SIZE, req.info, input, output);

        let uuid = Uuid::parse_str(&req.request_id).map_err(Error::InvaildUUID)?;
        if let Some(x) = self.dup.check_i32(user_id, &uuid) {
            return Ok(Response::new(x.into()));
        };

        if !(perm.super_user()) {
            return Err(Error::RequirePermission(RoleLv::Super).into());
        }

        let mut model: ActiveModel = Default::default();
        model.user_id = ActiveValue::Set(user_id);

        fill_active_model!(model, req.info, input, output, score);

        let model = model
            .save(self.db.deref())
            .await
            .map_err(Into::<Error>::into)?;

        self.dup.store_i32(user_id, uuid, model.id.clone().unwrap());

        tracing::debug!(id = model.id.clone().unwrap(), "testcase_created");

        Ok(Response::new(model.id.unwrap().into()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn update(&self, req: Request<UpdateTestcaseRequest>) -> Result<Response<()>, Status> {
        let (auth, req) = self.parse_request(req).await?;
        let (user_id, _perm) = auth.ok_or_default()?;

        check_exist_length!(LONG_ART_SIZE, req.info, input, output);

        let uuid = Uuid::parse_str(&req.request_id).map_err(Error::InvaildUUID)?;
        if self.dup.check_i32(user_id, &uuid).is_some() {
            return Ok(Response::new(()));
        };

        tracing::trace!(id = req.id.id);

        let mut model = Entity::write_filter(Entity::find_by_id(req.id), &auth)?
            .one(self.db.deref())
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB)?
            .into_active_model();

        fill_exist_active_model!(model, req.info, input, output, score);

        let model = model
            .update(self.db.deref())
            .await
            .map_err(Into::<Error>::into)?;

        self.dup.store_i32(user_id, uuid, model.id);

        Ok(Response::new(()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn remove(&self, req: Request<TestcaseId>) -> Result<Response<()>, Status> {
        let (auth, req) = self.parse_request(req).await?;

        let result = Entity::write_filter(Entity::delete_by_id(Into::<i32>::into(req.id)), &auth)?
            .exec(self.db.deref())
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
        let (auth, req) = self.parse_request(req).await?;
        let (user_id, perm) = auth.ok_or_default()?;

        if !perm.super_user() {
            return Err(Error::RequirePermission(RoleLv::Super).into());
        }

        let (problem, model) = try_join!(
            problem::Entity::read_by_id(req.problem_id.id, &auth)?.one(self.db.deref()),
            Entity::read_by_id(req.testcase_id.id, &auth)?.one(self.db.deref())
        )
        .map_err(Into::<Error>::into)?;

        let problem = problem.ok_or(Error::NotInDB)?;
        let model = model.ok_or(Error::NotInDB)?;

        if !(perm.admin()) {
            if problem.user_id != user_id {
                return Err(Error::UnownedAdd("problem").into());
            }
        }

        let mut model = model.into_active_model();
        model.problem_id = ActiveValue::Set(Some(req.problem_id.id));
        model
            .save(self.db.deref())
            .await
            .map_err(Into::<Error>::into)?;

        Ok(Response::new(()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn remove_from_problem(
        &self,
        req: Request<AddTestcaseToProblemRequest>,
    ) -> Result<Response<()>, Status> {
        let (auth, req) = self.parse_request(req).await?;

        let mut test = Entity::write_by_id(req.problem_id.id, &auth)?
            .columns([Column::Id, Column::ProblemId])
            .one(self.db.deref())
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB)?
            .into_active_model();

        test.problem_id = ActiveValue::Set(None);

        test.save(self.db.deref())
            .await
            .map_err(Into::<Error>::into)?;

        Ok(Response::new(()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn full_info_by_problem(
        &self,
        req: Request<AddTestcaseToProblemRequest>,
    ) -> Result<Response<TestcaseFullInfo>, Status> {
        let (auth, req) = self.parse_request(req).await?;

        tracing::debug!(
            problem_id = req.problem_id.id,
            testcase_id = req.testcase_id.id
        );

        let (_, perm) = auth.ok_or_default()?;

        if !perm.admin() {
            return Err(Error::RequirePermission(RoleLv::Root).into());
        }

        let parent: problem::IdModel =
            problem::Entity::related_read_by_id(&auth, Into::<i32>::into(req.problem_id), &self.db)
                .await?;

        let model = parent
            .upgrade()
            .find_related(Entity)
            .filter(Column::Id.eq(Into::<i32>::into(req.testcase_id)))
            .one(self.db.deref())
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB)?;

        Ok(Response::new(model.into()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn list_by_problem(
        &self,
        req: Request<ListByRequest>,
    ) -> Result<Response<ListTestcaseResponse>, Status> {
        let (auth, req) = self.parse_request(req).await?;

        let (rev, size) = split_rev(req.size);
        let size = bound!(size, 64);
        let offset = bound!(req.offset(), 1024);

        let (pager, models) = match req.request.ok_or(Error::NotInPayload("request"))? {
            list_by_request::Request::Create(create) => {
                ParentPaginator::new_fetch(
                    (create.parent_id, Default::default()),
                    &auth,
                    size,
                    offset,
                    create.start_from_end,
                    &self.db,
                )
                .await
            }
            list_by_request::Request::Pager(old) => {
                let pager: ParentPaginator = self.crypto.decode(old.session)?;
                pager.fetch(&auth, size, offset, rev, &self.db).await
            }
        }?;

        let next_session = self.crypto.encode(pager)?;
        let list = models.into_iter().map(|x| x.into()).collect();

        Ok(Response::new(ListTestcaseResponse { list, next_session }))
    }
}
