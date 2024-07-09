use super::tools::*;

use crate::util::time::into_prost;
use grpc::backend::announcement_server::*;
use grpc::backend::*;

use crate::entity::announcement::*;
use crate::entity::*;

use crate::NonZeroU32;

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
        // let (auth, rev, size, offset, pager) = parse_pager_param!(self, req);
        //
        // let (pager, models) = match pager {
        //     list_announcement_request::Request::Create(create) => {
        //         ColPaginator::new_fetch(
        //             (create.sort_by(), Default::default()),
        //             &auth,
        //             size,
        //             offset,
        //             create.start_from_end(),
        //             &self.db,
        //         )
        //         .in_current_span()
        //         .await
        //     }
        //     list_announcement_request::Request::Pager(old) => {
        //         let span = tracing::info_span!("paginate").or_current();
        //         let pager: ColPaginator = span.in_scope(|| self.crypto.decode(old.session))?;
        //         pager
        //             .fetch(&auth, size, offset, rev, &self.db)
        //             .instrument(span)
        //             .await
        //     }
        // }?;
        //
        // let remain = pager.remain(&auth, &self.db).in_current_span().await?;
        //
        // let next_session = self.crypto.encode(pager)?;
        // let list = models.into_iter().map(|x| x.into()).collect();
        //
        // Ok(Response::new(ListAnnouncementResponse {
        //     list,
        //     next_session,
        //     remain,
        // }))
        todo!()
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
        let (user_id, perm) = auth.ok_or_default()?;

        check_length!(SHORT_ART_SIZE, req.info, title);
        check_length!(LONG_ART_SIZE, req.info, content);

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
        let (user_id, _perm) = auth.ok_or_default()?;

        check_exist_length!(SHORT_ART_SIZE, req.info, title);
        check_exist_length!(LONG_ART_SIZE, req.info, content);

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
        let (user_id, perm) = auth.ok_or_default()?;

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
