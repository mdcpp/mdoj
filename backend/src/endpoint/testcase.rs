use super::endpoints::*;
use super::tools::*;

use crate::grpc::backend::testcase_set_server::*;
use crate::grpc::backend::*;

use entity::{test::*, *};

#[async_trait]
impl Filter for Entity {
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
        Err(Error::PremissionDeny("Can't write test"))
    }
}

#[async_trait]
impl ParentalFilter for Entity {
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
        Err(Error::PremissionDeny("Can't link test"))
    }
}

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
        todo!()
        // TestcaseInfo { id: value.id.into(), score:value.score }
    }
}

#[async_trait]
impl TestcaseSet for Arc<Server> {
    async fn create(
        &self,
        req: Request<CreateTestcaseRequest>,
    ) -> Result<Response<TestcaseId>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;

        let (user_id, perm) = auth.ok_or_default()?;

        let uuid = Uuid::parse_str(&req.request_id).map_err(|e| Error::InvaildUUID(e))?;
        if let Some(x) = self.dup.check(user_id, &uuid) {
            return Ok(Response::new(x.into()));
        };

        if !(perm.can_root() || perm.can_manage_problem()) {
            return Err(Error::PremissionDeny("Can't create test").into());
        }

        let mut model: ActiveModel = Default::default();
        model.user_id = ActiveValue::Set(user_id);

        fill_active_model!(model, req.info, input, output, score);

        let model = model.save(db).await.map_err(|x| Into::<Error>::into(x))?;

        self.dup.store(user_id, uuid, model.id.clone().unwrap());

        Ok(Response::new(model.id.unwrap().into()))
    }
    async fn update(&self, req: Request<UpdateTestcaseRequest>) -> Result<Response<()>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;

        let (user_id, perm) = auth.ok_or_default()?;

        let uuid = Uuid::parse_str(&req.request_id).map_err(|e| Error::InvaildUUID(e))?;
        if let Some(_) = self.dup.check(user_id, &uuid) {
            return Ok(Response::new(()));
        };

        if !(perm.can_root() || perm.can_manage_problem()) {
            return Err(Error::PremissionDeny("Can't update test").into());
        }

        let mut model = Entity::write_filter(Entity::find_by_id(req.id), &auth)?
            .one(db)
            .await
            .map_err(|x| Into::<Error>::into(x))?
            .ok_or(Error::NotInDB("test"))?
            .into_active_model();

        fill_exist_active_model!(model, req.info, input, output, score);

        let model = model.update(db).await.map_err(|x| Into::<Error>::into(x))?;

        self.dup.store(user_id, uuid, model.id);

        Ok(Response::new(()))
    }
    async fn remove(&self, req: Request<TestcaseId>) -> Result<Response<()>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;

        Entity::write_filter(Entity::delete_by_id(Into::<i32>::into(req.id)), &auth)?
            .exec(db)
            .await
            .map_err(|x| Into::<Error>::into(x))?;

        Ok(Response::new(()))
    }
    async fn link(&self, req: Request<TestcaseLink>) -> Result<Response<()>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;

        let (_, perm) = auth.ok_or_default()?;

        if !(perm.can_root() || perm.can_link()) {
            return Err(Error::PremissionDeny("Can't link test").into());
        }

        let mut test = Entity::link_filter(Entity::find_by_id(req.problem_id.id.clone()), &auth)?
            .columns([Column::Id, Column::ProblemId])
            .one(db)
            .await
            .map_err(|x| Into::<Error>::into(x))?
            .ok_or(Error::NotInDB("test"))?
            .into_active_model();

        test.problem_id = ActiveValue::Set(Some(req.problem_id.id));

        test.save(db).await.map_err(|x| Into::<Error>::into(x))?;

        Ok(Response::new(()))
    }
    async fn unlink(&self, req: Request<TestcaseLink>) -> Result<Response<()>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;

        let (_, perm) = auth.ok_or_default()?;

        if !(perm.can_root() || perm.can_link()) {
            return Err(Error::PremissionDeny("Can't link test").into());
        }

        let mut test = Entity::link_filter(Entity::find_by_id(req.problem_id.id.clone()), &auth)?
            .columns([Column::Id, Column::ProblemId])
            .one(db)
            .await
            .map_err(|x| Into::<Error>::into(x))?
            .ok_or(Error::NotInDB("test"))?
            .into_active_model();

        test.problem_id = ActiveValue::Set(None);

        test.save(db).await.map_err(|x| Into::<Error>::into(x))?;

        Ok(Response::new(()))
    }
    async fn full_info_by_problem(
        &self,
        req: Request<TestcaseLink>,
    ) -> Result<Response<TestcaseFullInfo>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;

        let (_, perm) = auth.ok_or_default()?;

        if !(perm.can_root() || perm.can_manage_problem()) {
            return Err(
                Error::PremissionDeny("input and output field of problem is protected").into(),
            );
        }

        let parent = auth
            .get_user(db)
            .await?
            .find_related(problem::Entity)
            .columns([problem::Column::Id])
            .one(db)
            .await
            .map_err(|x| Into::<Error>::into(x))?
            .ok_or(Error::NotInDB("problem"))?;

        let model = parent
            .find_related(Entity)
            .filter(Column::Id.eq(Into::<i32>::into(req.problem_id)))
            .one(db)
            .await
            .map_err(|x| Into::<Error>::into(x))?
            .ok_or(Error::NotInDB("test"))?;

        Ok(Response::new(model.into()))
    }
    async fn list_by_problem(
        &self,
        req: Request<ListByRequest>,
    ) -> Result<Response<ListTestcaseResponse>, Status> {
        let (auth, req) = self.parse_request(req).await?;

        let mut reverse = false;
        let mut pager: Pager<Entity> = match req.request.ok_or(Error::NotInPayload("request"))? {
            list_by_request::Request::ParentId(ppk) => Pager::parent_search(ppk),
            list_by_request::Request::Pager(old) => {
                reverse = old.reverse;
                <Pager<Entity> as HasParentPager<problem::Entity, Entity>>::from_raw(old.session)?
            }
        };

        let list = pager
            .fetch(req.size, req.offset.unwrap_or_default(), reverse, &auth)
            .await?
            .into_iter()
            .map(|x| x.into())
            .collect();

        let next_session = pager.into_raw();

        Ok(Response::new(ListTestcaseResponse {
            list,
            next_session,
        }))
    }
}
