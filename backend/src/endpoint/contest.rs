use super::endpoints::*;
use super::tools::*;

use crate::grpc::backend::contest_set_server::*;
use crate::grpc::backend::*;
use crate::grpc::into_chrono;
use crate::grpc::into_prost;
use entity::{contest::*, *};
use sea_orm::QueryOrder;

#[async_trait]
impl Filter for Entity {
    fn write_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
        if let Some(perm) = auth.user_perm() {
            if perm.can_root() {
                return Ok(query);
            }
            if perm.can_manage_contest() {
                let user_id = auth.user_id().unwrap();
                return Ok(query.filter(Column::Hoster.eq(user_id)));
            }
        }
        Err(Error::PremissionDeny("Can't write contest"))
    }
    fn read_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
        if let Some(perm) = auth.user_perm() {
            if perm.can_link() || perm.can_root() || perm.can_manage_contest() {
                return Ok(query);
            }
        }
        Ok(query.filter(Column::Public.eq(true)))
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
                return Ok(query.filter(Column::Hoster.eq(user_id)));
            }
        }
        Err(Error::PremissionDeny("Can't link test"))
    }
}

impl From<i32> for ContestId {
    fn from(value: i32) -> Self {
        Self { id: value }
    }
}
impl From<ContestId> for i32 {
    fn from(value: ContestId) -> Self {
        value.id
    }
}

impl From<Model> for ContestFullInfo {
    fn from(value: Model) -> Self {
        ContestFullInfo {
            info: value.clone().into(),
            content: value.content,
            hoster: value.hoster.into(),
        }
    }
}

impl From<user_contest::Model> for UserRank {
    fn from(value: user_contest::Model) -> Self {
        UserRank {
            user_id: value.user_id.into(),
            score: value.score,
        }
    }
}

impl From<Model> for ContestInfo {
    fn from(value: Model) -> Self {
        ContestInfo {
            id: value.id.into(),
            title: value.title,
            begin: into_prost(value.begin),
            end: into_prost(value.end),
            need_password: value.password.is_some(),
        }
    }
}

#[async_trait]
impl ContestSet for Arc<Server> {
    #[instrument(skip_all, level = "debug")]
    async fn list(
        &self,
        req: Request<ListRequest>,
    ) -> Result<Response<ListContestResponse>, Status> {
        let (auth, req) = self.parse_request(req).await?;

        let mut reverse = false;
        let mut pager: Pager<Entity> = match req.request.ok_or(Error::NotInPayload("request"))? {
            list_request::Request::Create(create) => {
                Pager::sort_search(create.sort_by(), create.reverse)
            }
            list_request::Request::Pager(old) => {
                reverse = old.reverse;
                <Pager<Entity> as NoParentPager<Entity>>::from_raw(old.session, &self)?
            }
        };

        let list = pager
            .fetch(req.size, req.offset.unwrap_or_default(), reverse, &auth)
            .await?
            .into_iter()
            .map(|x| x.into())
            .collect();

        let next_session = pager.into_raw(&self);

        Ok(Response::new(ListContestResponse { list, next_session }))
    }
    #[instrument(skip_all, level = "debug")]
    async fn search_by_text(
        &self,
        req: Request<TextSearchRequest>,
    ) -> Result<Response<ListContestResponse>, Status> {
        let (auth, req) = self.parse_request(req).await?;

        let mut reverse = false;
        let mut pager: Pager<Entity> = match req.request.ok_or(Error::NotInPayload("request"))? {
            text_search_request::Request::Text(create) => Pager::text_search(create),
            text_search_request::Request::Pager(old) => {
                reverse = old.reverse;
                <Pager<_> as NoParentPager<Entity>>::from_raw(old.session, &self)?
            }
        };

        let list = pager
            .fetch(req.size, req.offset.unwrap_or_default(), reverse, &auth)
            .await?
            .into_iter()
            .map(|x| x.into())
            .collect();

        let next_session = pager.into_raw(&self);

        Ok(Response::new(ListContestResponse { list, next_session }))
    }
    #[instrument(skip_all, level = "debug")]
    async fn full_info(
        &self,
        req: Request<ContestId>,
    ) -> Result<Response<ContestFullInfo>, Status> {
        let db = DB.get().unwrap();
        let (_, req) = self.parse_request(req).await?;

        let query = Entity::find_by_id::<i32>(req.into()).filter(Column::Public.eq(true));
        let model = query
            .one(db)
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB("contest"))?;

        Ok(Response::new(model.into()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn create(
        &self,
        req: Request<CreateContestRequest>,
    ) -> Result<Response<ContestId>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;

        let (user_id, perm) = auth.ok_or_default()?;

        let uuid = Uuid::parse_str(&req.request_id).map_err(Error::InvaildUUID)?;
        if let Some(x) = self.dup.check_i32(user_id, &uuid) {
            return Ok(Response::new(x.into()));
        };

        if !(perm.can_root() || perm.can_manage_contest()) {
            return Err(Error::PremissionDeny("Can't create contest").into());
        }

        let mut model: ActiveModel = Default::default();
        model.hoster = ActiveValue::Set(user_id);

        fill_active_model!(model, req.info, title, content, tags);

        let password: Vec<u8> = req
            .info
            .password
            .map(|a| self.crypto.hash(&a))
            .ok_or(Error::NotInPayload("password"))?
            .into();
        model.password = ActiveValue::Set(Some(password));

        model.begin = ActiveValue::Set(into_chrono(req.info.begin));
        model.end = ActiveValue::Set(into_chrono(req.info.end));

        let model = model.save(db).await.map_err(Into::<Error>::into)?;

        self.dup.store_i32(user_id, uuid, model.id.clone().unwrap());

        self.metrics.contest.add(1, &[]);
        tracing::debug!(id = model.id.clone().unwrap());

        Ok(Response::new(model.id.unwrap().into()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn update(&self, req: Request<UpdateContestRequest>) -> Result<Response<()>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;

        let (user_id, perm) = auth.ok_or_default()?;

        let uuid = Uuid::parse_str(&req.request_id).map_err(Error::InvaildUUID)?;
        if self.dup.check_i32(user_id, &uuid).is_some() {
            return Ok(Response::new(()));
        };

        if !(perm.can_root() || perm.can_manage_contest()) {
            return Err(Error::PremissionDeny("Can't update contest").into());
        }

        let mut model = Entity::write_filter(Entity::find_by_id(req.id), &auth)?
            .one(db)
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB("contest"))?;

        if let Some(src) = req.info.password {
            if let Some(tar) = model.password.as_ref() {
                if auth.is_root() || self.crypto.hash_eq(&src, tar) {
                    let hash = self.crypto.hash(&src).into();
                    model.password = Some(hash);
                } else {
                    return Err(Error::PremissionDeny(
                        "password should match in order to update password!",
                    )
                    .into());
                }
            }
        }

        let mut model = model.into_active_model();

        fill_exist_active_model!(model, req.info, title, content, tags);
        if let Some(x) = req.info.begin {
            model.begin = ActiveValue::Set(into_chrono(x));
        }
        if let Some(x) = req.info.end {
            model.end = ActiveValue::Set(into_chrono(x));
        }

        let model = model.update(db).await.map_err(Into::<Error>::into)?;

        self.dup.store_i32(user_id, uuid, model.id);

        Ok(Response::new(()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn remove(&self, req: Request<ContestId>) -> Result<Response<()>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;

        Entity::write_filter(Entity::delete_by_id(Into::<i32>::into(req.id)), &auth)?
            .exec(db)
            .await
            .map_err(Into::<Error>::into)?;

        self.metrics.contest.add(-1, &[]);
        tracing::debug!(id = req.id, "contest_remove");

        Ok(Response::new(()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn join(&self, req: Request<JoinContestRequest>) -> Result<Response<()>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;

        let (user_id, _) = auth.ok_or_default()?;

        let model = Entity::read_filter(Entity::find_by_id(req.id.id), &auth)?
            .one(db)
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB("contest"))?;

        let empty_password = "".to_string();
        if let Some(tar) = model.password {
            if (!auth.is_root())
                && (!self
                    .crypto
                    .hash_eq(req.password.as_ref().unwrap_or(&empty_password), &tar))
                && model.public
            {
                return Err(Error::PremissionDeny("contest password mismatch").into());
            }
        }

        let pivot = user_contest::ActiveModel {
            user_id: ActiveValue::Set(user_id),
            contest_id: ActiveValue::Set(model.id),
            ..Default::default()
        };

        pivot.save(db).await.map_err(Into::<Error>::into)?;

        tracing::debug!(user_id = user_id, contest_id = req.id.id);

        Ok(Response::new(()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn exit(&self, req: Request<ContestId>) -> Result<Response<()>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;
        let (user_id, _) = auth.ok_or_default()?;

        user_contest::Entity::delete_many()
            .filter(user_contest::Column::UserId.eq(user_id))
            .filter(user_contest::Column::ContestId.eq(req.id))
            .exec(db)
            .await
            .map_err(Into::<Error>::into)?;

        tracing::debug!(user_id = user_id, contest_id = req.id, "user_exit");

        Ok(Response::new(()))
    }

    #[doc = " return up to 10 user"]
    #[instrument(skip_all, level = "debug")]
    async fn rank(&self, req: Request<ContestId>) -> Result<Response<Users>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;
        let (user_id, _) = auth.ok_or_default()?;

        let user = user::Entity::find_by_id(user_id)
            .column(user::Column::Id)
            .one(db)
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB("user"))?;

        let contest = user
            .find_related(Entity)
            .column(Column::Id)
            .filter(Column::Id.eq(Into::<i32>::into(req.id)))
            .one(db)
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB("user"))?;

        let list = user_contest::Entity::find()
            .filter(user_contest::Column::ContestId.eq(contest.id))
            .order_by_desc(user_contest::Column::Score)
            .limit(10)
            .all(db)
            .await
            .map_err(Into::<Error>::into)?;

        let list: Vec<UserRank> = list.into_iter().map(|x| x.into()).collect();

        Ok(Response::new(Users { list }))
    }
}
