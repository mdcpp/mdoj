use super::tools::*;

use grpc::{backend::announcement_server::*, backend::*};

use crate::{
    entity::announcement::{Paginator, *},
    entity::*,
    util::time::into_prost,
    NonZeroU32,
};

impl From<Model> for AnnouncementFullInfo {
    fn from(value: Model) -> Self {
        AnnouncementFullInfo {
            info: AnnouncementInfo {
                id: value.id.into(),
                title: value.title,
                update_date: into_prost(value.update_at),
            },
            author_id: value.user_id.into(),
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
            update_date: into_prost(value.update_at),
        }
    }
}

impl From<PartialModel> for AnnouncementInfo {
    fn from(value: PartialModel) -> Self {
        AnnouncementInfo {
            id: value.id.into(),
            title: value.title,
            update_date: into_prost(value.update_at),
        }
    }
}

#[async_trait]
impl Announcement for ArcServer {
    #[instrument(skip_all, level = "debug")]
    async fn list(
        &self,
        req: Request<ListAnnouncementRequest>,
    ) -> Result<Response<ListAnnouncementResponse>, Status> {
        let (auth, req) = self
            .parse_request_fn(req, |req| {
                ((req.size as u64) + req.offset.saturating_abs() as u64 / 5 + 2)
                    .try_into()
                    .unwrap_or(u32::MAX)
            })
            .await?;

        req.bound_check()?;

        let paginator = match req.request.ok_or(Error::NotInPayload("request"))? {
            list_announcement_request::Request::Create(create) => {
                let query = create.query.unwrap_or_default();
                let start_from_end = create.order == Order::Descend as i32;
                if let Some(text) = query.text {
                    Paginator::new_text(text, start_from_end)
                } else if let Some(sort) = query.sort_by {
                    Paginator::new_sort(sort.try_into().unwrap_or_default(), start_from_end)
                } else if let Some(parent) = query.contest_id {
                    Paginator::new_parent(parent, start_from_end)
                } else {
                    Paginator::new(start_from_end)
                }
            }
            list_announcement_request::Request::Paginator(x) => self.crypto.decode(x)?,
        };
        let mut paginator = paginator.with_auth(&auth).with_db(&self.db);

        let list = paginator.fetch(req.size, req.offset).await?;
        let remain = paginator.remain().await?;

        let paginator = paginator.into_inner();

        Ok(Response::new(ListAnnouncementResponse {
            list: list.into_iter().map(Into::into).collect(),
            paginator: self.crypto.encode(paginator)?,
            remain,
        }))
    }
    #[instrument(skip_all, level = "debug")]
    async fn full_info(&self, req: Request<Id>) -> Result<Response<AnnouncementFullInfo>, Status> {
        let (auth, req) = self
            .parse_request_n(req, NonZeroU32!(5))
            .in_current_span()
            .await?;

        tracing::debug!(announcement_id = req.id);

        let query = Entity::read_filter(Entity::find_by_id::<i32>(req.into()), &auth)?;
        let model = query
            .one(self.db.deref())
            .instrument(info_span!("fetch").or_current())
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB)?;

        Ok(Response::new(model.into()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn create(
        &self,
        req: Request<CreateAnnouncementRequest>,
    ) -> Result<Response<Id>, Status> {
        let (auth, req) = self
            .parse_request_n(req, NonZeroU32!(5))
            .in_current_span()
            .await?;
        let (user_id, perm) = auth.auth_or_guest()?;

        req.bound_check()?;

        let uuid = Uuid::parse_str(&req.request_id).map_err(Error::InvaildUUID)?;
        if let Some(x) = self.dup.check::<Id>(user_id, uuid) {
            return Ok(Response::new(x));
        };

        if perm.super_user() {
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

        let id: Id = model.id.clone().unwrap().into();

        self.dup.store(user_id, uuid, id.clone());
        tracing::debug!(id = id.id, "announcement_created");

        Ok(Response::new(id))
    }
    #[instrument(skip_all, level = "debug")]
    async fn update(
        &self,
        req: Request<UpdateAnnouncementRequest>,
    ) -> Result<Response<()>, Status> {
        let (auth, req) = self
            .parse_request_n(req, NonZeroU32!(5))
            .in_current_span()
            .await?;
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
    async fn remove(&self, req: Request<Id>) -> Result<Response<()>, Status> {
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

        Ok(Response::new(()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn add_to_contest(
        &self,
        req: Request<AddAnnouncementToContestRequest>,
    ) -> Result<Response<()>, Status> {
        let (auth, req) = self
            .parse_request_n(req, NonZeroU32!(5))
            .in_current_span()
            .await?;
        let (user_id, perm) = auth.auth_or_guest()?;

        if !perm.super_user() {
            return Err(Error::RequirePermission(RoleLv::Super).into());
        }

        let (contest, model) = tokio::try_join!(
            contest::Entity::read_by_id(req.contest_id, &auth)?
                .one(self.db.deref())
                .instrument(info_span!("fetch_parent").or_current()),
            Entity::read_by_id(req.announcement_id, &auth)?
                .one(self.db.deref())
                .instrument(info_span!("fetch_child").or_current())
        )
        .map_err(Into::<Error>::into)?;

        let contest = contest.ok_or(Error::NotInDB)?;
        let model = model.ok_or(Error::NotInDB)?;

        if !perm.admin() {
            if contest.hoster != user_id {
                return Err(Error::NotInDB.into());
            }
            if model.user_id != user_id {
                return Err(Error::NotInDB.into());
            }
        }

        let mut model = model.into_active_model();
        model.contest_id = ActiveValue::Set(Some(req.contest_id));
        model
            .update(self.db.deref())
            .instrument(info_span!("update").or_current())
            .await
            .map_err(atomic_fail)?;

        Ok(Response::new(()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn remove_from_contest(
        &self,
        req: Request<AddAnnouncementToContestRequest>,
    ) -> Result<Response<()>, Status> {
        let (auth, req) = self
            .parse_request_n(req, NonZeroU32!(5))
            .in_current_span()
            .await?;

        let mut announcement = Entity::write_by_id(req.announcement_id, &auth)?
            .columns([Column::Id, Column::ContestId])
            .one(self.db.deref())
            .instrument(info_span!("fetcg").or_current())
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB)?
            .into_active_model();

        announcement.contest_id = ActiveValue::Set(None);

        announcement
            .save(self.db.deref())
            .instrument(info_span!("remove").or_current())
            .await
            .map_err(Into::<Error>::into)?;

        Ok(Response::new(()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn publish(&self, req: Request<Id>) -> Result<Response<()>, Status> {
        let (auth, req) = self
            .parse_request_n(req, NonZeroU32!(5))
            .in_current_span()
            .await?;
        let perm = auth.user_perm();

        tracing::debug!(id = req.id);

        if !perm.admin() {
            return Err(Error::RequirePermission(RoleLv::Root).into());
        }

        let mut model = Entity::find_by_id(Into::<i32>::into(req))
            .columns([Column::Id, Column::ContestId])
            .one(self.db.deref())
            .instrument(info_span!("fetch").or_current())
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB)?
            .into_active_model();

        model.public = ActiveValue::Set(true);

        model
            .update(self.db.deref())
            .instrument(info_span!("update").or_current())
            .await
            .map_err(atomic_fail)?;

        Ok(Response::new(()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn unpublish(&self, req: Request<Id>) -> Result<Response<()>, Status> {
        let (auth, req) = self
            .parse_request_n(req, NonZeroU32!(5))
            .in_current_span()
            .await?;
        let perm = auth.user_perm();

        tracing::debug!(id = req.id);

        if !perm.admin() {
            return Err(Error::RequirePermission(RoleLv::Root).into());
        }

        let mut model = Entity::find_by_id(Into::<i32>::into(req))
            .columns([Column::Id, Column::ContestId])
            .one(self.db.deref())
            .instrument(info_span!("fetch").or_current())
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB)?
            .into_active_model();

        model.public = ActiveValue::Set(false);

        model
            .update(self.db.deref())
            .instrument(info_span!("update").or_current())
            .await
            .map_err(atomic_fail)?;

        Ok(Response::new(()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn full_info_by_contest(
        &self,
        req: Request<AddAnnouncementToContestRequest>,
    ) -> Result<Response<AnnouncementFullInfo>, Status> {
        let (auth, req) = self
            .parse_request_n(req, NonZeroU32!(5))
            .in_current_span()
            .await?;

        let parent: contest::IdModel =
            contest::Entity::related_read_by_id(&auth, Into::<i32>::into(req.contest_id), &self.db)
                .in_current_span()
                .await?;
        let model = parent
            .upgrade()
            .find_related(Entity)
            .filter(Column::Id.eq(Into::<i32>::into(req.announcement_id)))
            .one(self.db.deref())
            .instrument(info_span!("fetch").or_current())
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB)?;

        Ok(Response::new(model.into()))
    }
}
