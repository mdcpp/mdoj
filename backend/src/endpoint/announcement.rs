use super::endpoints::*;
use super::tools::*;

use crate::grpc::backend::announcement_set_server::*;
use crate::grpc::backend::*;
use crate::grpc::into_prost;

use entity::announcement::*;
use entity::*;
use sea_orm::QueryTrait;

impl From<i32> for AnnouncementId {
    fn from(value: i32) -> Self {
        Self { id: value }
    }
}

impl From<AnnouncementId> for i32 {
    fn from(value: AnnouncementId) -> Self {
        value.id
    }
}

impl From<Model> for AnnouncementFullInfo {
    fn from(value: Model) -> Self {
        AnnouncementFullInfo {
            info: AnnouncementInfo {
                id: value.id.into(),
                title: value.title,
                upload_date: into_prost(value.create_at),
            },
            author: value.user_id.into(),
            content: value.content,
            public: value.public,
        }
    }
}

impl From<Model> for AnnouncementInfo {
    fn from(value: Model) -> Self {
        AnnouncementInfo {
            id: value.id.into(),
            title: value.title,
            upload_date: into_prost(value.create_at),
        }
    }
}

#[async_trait]
impl AnnouncementSet for Arc<Server> {
    #[instrument(skip_all, level = "debug")]
    async fn list(
        &self,
        req: Request<ListAnnouncementRequest>,
    ) -> Result<Response<ListAnnouncementResponse>, Status> {
        let (auth, req) = self.parse_request(req).await?;

        let mut reverse = false;
        let mut pager: Pager<Entity> = match req.request.ok_or(Error::NotInPayload("request"))? {
            list_announcement_request::Request::Create(create) => {
                Pager::sort_search(create.sort_by(), create.reverse)
            }
            list_announcement_request::Request::Pager(old) => {
                reverse = old.reverse;
                <Pager<Entity> as HasParentPager<contest::Entity, Entity>>::from_raw(
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

        Ok(Response::new(ListAnnouncementResponse {
            list,
            next_session,
        }))
    }
    #[instrument(skip_all, level = "debug")]
    async fn search_by_text(
        &self,
        req: Request<TextSearchRequest>,
    ) -> Result<Response<ListAnnouncementResponse>, Status> {
        let (auth, req) = self.parse_request(req).await?;

        let mut reverse = false;
        let mut pager: Pager<Entity> = match req.request.ok_or(Error::NotInPayload("request"))? {
            text_search_request::Request::Text(create) => {
                tracing::trace!(search = create);
                Pager::text_search(create)
            }
            text_search_request::Request::Pager(old) => {
                reverse = old.reverse;
                <Pager<_> as HasParentPager<contest::Entity, Entity>>::from_raw(old.session, self)?
            }
        };

        let list = pager
            .fetch(req.size, req.offset.unwrap_or_default(), reverse, &auth)
            .await?
            .into_iter()
            .map(|x| x.into())
            .collect();

        let next_session = pager.into_raw(self);

        Ok(Response::new(ListAnnouncementResponse {
            list,
            next_session,
        }))
    }
    #[instrument(skip_all, level = "debug")]
    async fn full_info(
        &self,
        req: Request<AnnouncementId>,
    ) -> Result<Response<AnnouncementFullInfo>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;

        tracing::debug!(announcement_id = req.id);

        let query = Entity::read_filter(Entity::find_by_id::<i32>(req.into()), &auth)?;
        let model = query
            .one(db)
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB(Entity::DEBUG_NAME))?;

        Ok(Response::new(model.into()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn create(
        &self,
        req: Request<CreateAnnouncementRequest>,
    ) -> Result<Response<AnnouncementId>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;
        let (user_id, perm) = auth.ok_or_default()?;

        let uuid = Uuid::parse_str(&req.request_id).map_err(Error::InvaildUUID)?;
        if let Some(x) = self.dup.check_i32(user_id, &uuid) {
            return Ok(Response::new(x.into()));
        };

        if !(perm.can_root() || perm.can_manage_announcement()) {
            return Err(Error::RequirePermission(Entity::DEBUG_NAME).into());
        }

        let mut model: ActiveModel = Default::default();
        model.user_id = ActiveValue::Set(user_id);

        fill_active_model!(model, req.info, title, content);

        let model = model.save(db).await.map_err(Into::<Error>::into)?;

        self.dup.store_i32(user_id, uuid, model.id.clone().unwrap());

        tracing::debug!(id = model.id.clone().unwrap(), "announcement_created");

        Ok(Response::new(model.id.unwrap().into()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn update(
        &self,
        req: Request<UpdateAnnouncementRequest>,
    ) -> Result<Response<()>, Status> {
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
            .ok_or(Error::NotInDB(Entity::DEBUG_NAME))?
            .into_active_model();

        fill_exist_active_model!(model, req.info, title, content);

        let model = model.update(db).await.map_err(Into::<Error>::into)?;

        self.dup.store_i32(user_id, uuid, model.id);

        Ok(Response::new(()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn remove(&self, req: Request<AnnouncementId>) -> Result<Response<()>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;

        Entity::write_filter(Entity::delete_by_id(Into::<i32>::into(req.id)), &auth)?
            .exec(db)
            .await
            .map_err(Into::<Error>::into)?;

        tracing::debug!(id = req.id);

        Ok(Response::new(()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn add_to_contest(
        &self,
        req: Request<AddAnnouncementToContestRequest>,
    ) -> Result<Response<()>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;
        let (user_id, perm) = auth.ok_or_default()?;

        let (contest, model) = try_join!(
            spawn(contest::Entity::read_by_id(req.contest_id.id, &auth)?.one(db)),
            spawn(Entity::read_by_id(req.announcement_id.id, &auth)?.one(db))
        )
        .unwrap();

        let contest = contest
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB("contest"))?;
        let model = model
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB(Entity::DEBUG_NAME))?;

        if !(perm.can_root() || perm.can_link()) {
            if contest.hoster != user_id {
                return Err(Error::NotInDB("contest").into());
            }
            if model.user_id != user_id {
                return Err(Error::NotInDB(Entity::DEBUG_NAME).into());
            }
            if !perm.can_manage_contest() {
                return Err(Error::RequirePermission("Contest").into());
            }
        }

        let mut model = model.into_active_model();
        model.contest_id = ActiveValue::Set(Some(req.contest_id.id));
        model.save(db).await.map_err(Into::<Error>::into)?;

        Ok(Response::new(()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn remove_from_contest(
        &self,
        req: Request<AddAnnouncementToContestRequest>,
    ) -> Result<Response<()>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;

        let (_, perm) = auth.ok_or_default()?;

        if !(perm.can_root() || perm.can_link()) {
            return Err(Error::RequirePermission("TODO").into());
        }

        let mut announcement = Entity::write_by_id(req.announcement_id, &auth)?
            .columns([Column::Id, Column::ContestId])
            .one(db)
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB(Entity::DEBUG_NAME))?
            .into_active_model();

        announcement.contest_id = ActiveValue::Set(None);

        announcement.save(db).await.map_err(Into::<Error>::into)?;

        Ok(Response::new(()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn publish(&self, req: Request<AnnouncementId>) -> Result<Response<()>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;

        tracing::debug!(id = req.id);

        if !auth.is_root() {
            return Err(Error::RequirePermission("Root").into());
        }

        let mut announcement = Entity::find_by_id(Into::<i32>::into(req))
            .columns([Column::Id, Column::ContestId])
            .one(db)
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB("announcement"))?
            .into_active_model();

        announcement.public = ActiveValue::Set(true);

        announcement.save(db).await.map_err(Into::<Error>::into)?;

        Ok(Response::new(()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn unpublish(&self, req: Request<AnnouncementId>) -> Result<Response<()>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;

        tracing::debug!(id = req.id);

        if !auth.is_root() {
            return Err(Error::RequirePermission("Root").into());
        }

        let mut announcement = Entity::find_by_id(Into::<i32>::into(req))
            .columns([Column::Id, Column::ContestId])
            .one(db)
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB("announcement"))?
            .into_active_model();

        announcement.public = ActiveValue::Set(false);

        announcement.save(db).await.map_err(Into::<Error>::into)?;

        Ok(Response::new(()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn full_info_by_contest(
        &self,
        req: Request<AddAnnouncementToContestRequest>,
    ) -> Result<Response<AnnouncementFullInfo>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;

        let parent = auth
            .get_user(db)
            .await?
            .find_related(contest::Entity)
            .select_only()
            .columns([contest::Column::Id])
            .filter(contest::Column::Id.eq(Into::<i32>::into(req.contest_id)))
            .one(db)
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB("contest"))?;

        let model = parent
            .find_related(Entity)
            .filter(Column::Id.eq(Into::<i32>::into(req.announcement_id)))
            .one(db)
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB(Entity::DEBUG_NAME))?;

        Ok(Response::new(model.into()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn list_by_contest(
        &self,
        req: Request<ListByRequest>,
    ) -> Result<Response<ListAnnouncementResponse>, Status> {
        let (auth, req) = self.parse_request(req).await?;

        let mut reverse = false;
        let mut pager: Pager<Entity> = match req.request.ok_or(Error::NotInPayload("request"))? {
            list_by_request::Request::ParentId(ppk) => {
                tracing::debug!(id = ppk);
                Pager::parent_sorted_search(ppk, AnnouncementSortBy::UploadDate, false)
            }
            list_by_request::Request::Pager(old) => {
                reverse = old.reverse;
                <Pager<Entity> as HasParentPager<contest::Entity, Entity>>::from_raw(
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

        Ok(Response::new(ListAnnouncementResponse {
            list,
            next_session,
        }))
    }
}
