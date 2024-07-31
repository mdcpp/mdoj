use super::*;

use grpc::backend::announcement_server::*;

use crate::{
    entity::announcement::{Paginator, *},
    entity::contest,
    util::time::into_prost,
};

impl<'a> From<WithAuth<'a, Model>> for AnnouncementFullInfo {
    fn from(value: WithAuth<'a, Model>) -> Self {
        let model = value.1;
        let writable = Entity::writable(&model, value.0);
        AnnouncementFullInfo {
            info: AnnouncementInfo {
                id: model.id,
                title: model.title,
                update_date: into_prost(model.update_at),
            },
            author_id: model.user_id,
            content: model.content,
            public: model.public,
            writable,
        }
    }
}

impl<'a> WithAuthTrait for Model {}

impl From<Model> for AnnouncementInfo {
    fn from(value: Model) -> Self {
        AnnouncementInfo {
            id: value.id,
            title: value.title,
            update_date: into_prost(value.update_at),
        }
    }
}

impl From<PartialModel> for AnnouncementInfo {
    fn from(value: PartialModel) -> Self {
        AnnouncementInfo {
            id: value.id,
            title: value.title,
            update_date: into_prost(value.update_at),
        }
    }
}

#[async_trait]
impl Announcement for ArcServer {
    #[instrument(
        skip_all,
        level = "info",
        name = "endpoint.Announcement.list",
        err(level = "debug", Display)
    )]
    async fn list(
        &self,
        req: Request<ListAnnouncementRequest>,
    ) -> Result<Response<ListAnnouncementResponse>, Status> {
        let (auth, req) = self.rate_limit(req).in_current_span().await?;

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

        let list = paginator
            .fetch(req.size, req.offset)
            .in_current_span()
            .await?;
        let remain = paginator.remain().in_current_span().await?;

        let paginator = paginator.into_inner();

        Ok(Response::new(ListAnnouncementResponse {
            list: list.into_iter().map(Into::into).collect(),
            paginator: self.crypto.encode(paginator)?,
            remain,
        }))
    }
    #[instrument(
        skip_all,
        level = "info",
        name = "endpoint.Announcement.full_info",
        err(level = "debug", Display)
    )]
    async fn full_info(&self, req: Request<Id>) -> Result<Response<AnnouncementFullInfo>, Status> {
        let (auth, req) = self.rate_limit(req).in_current_span().await?;

        let query = Entity::find_by_id::<i32>(req.into())
            .with_auth(&auth)
            .read()?;
        let model = query
            .one(self.db.deref())
            .instrument(info_span!("fetch").or_current())
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB)?;

        Ok(Response::new(model.with_auth(&auth).into()))
    }
    #[instrument(
        skip_all,
        level = "info",
        name = "endpoint.Announcement.full_info_by_contest",
        err(level = "debug", Display)
    )]
    async fn full_info_by_contest(
        &self,
        req: Request<ListAnnouncementByContestRequest>,
    ) -> Result<Response<AnnouncementFullInfo>, Status> {
        let (auth, req) = self.rate_limit(req).in_current_span().await?;

        let parent: contest::IdModel =
            contest::Entity::related_read_by_id(&auth, req.contest_id, &self.db)
                .in_current_span()
                .await?;
        let model = parent
            .upgrade()
            .find_related(Entity)
            .filter(Column::Id.eq(req.announcement_id))
            .one(self.db.deref())
            .instrument(info_span!("fetch").or_current())
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB)?;

        Ok(Response::new(model.with_auth(&auth).into()))
    }
    #[instrument(
        skip_all,
        level = "info",
        name = "endpoint.Announcement.create",
        err(level = "debug", Display)
    )]
    async fn create(
        &self,
        req: Request<CreateAnnouncementRequest>,
    ) -> Result<Response<Id>, Status> {
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

            tracing::info!(count.announcement = 1, id = id);

            Ok(id.into())
        })
        .await
        .with_grpc()
        .into()
    }
    #[instrument(
        skip_all,
        level = "info",
        name = "endpoint.Announcement.update",
        err(level = "debug", Display)
    )]
    async fn update(
        &self,
        req: Request<UpdateAnnouncementRequest>,
    ) -> Result<Response<()>, Status> {
        let (auth, req) = self.rate_limit(req).in_current_span().await?;
        req.bound_check()?;

        req.get_or_insert(|req| async move {
            tracing::trace!(id = req.id);

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
        name = "endpoint.Announcement.remove",
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
                tracing::info!(counter.announcement = -1, id = req.id);
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
        name = "endpoint.Announcement.add_to_contest",
        err(level = "debug", Display)
    )]
    async fn add_to_contest(
        &self,
        req: Request<AddAnnouncementToContestRequest>,
    ) -> Result<Response<()>, Status> {
        let (auth, req) = self.rate_limit(req).in_current_span().await?;
        auth.perm().super_user()?;

        req.get_or_insert(|req| async move {
            let (contest, model) = tokio::try_join!(
                contest::Entity::write_by_id(req.contest_id, &auth)?
                    .one(self.db.deref())
                    .instrument(info_span!("fetch_parent").or_current()),
                Entity::write_by_id(req.announcement_id, &auth)?
                    .one(self.db.deref())
                    .instrument(info_span!("fetch_child").or_current())
            )
            .map_err(Into::<Error>::into)?;

            contest.ok_or(Error::NotInDB)?;
            let mut model = model.ok_or(Error::NotInDB)?.into_active_model();

            model.contest_id = ActiveValue::Set(Some(req.contest_id));
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
        name = "endpoint.Announcement.remove_from_contest",
        err(level = "debug", Display)
    )]
    async fn remove_from_contest(
        &self,
        req: Request<AddAnnouncementToContestRequest>,
    ) -> Result<Response<()>, Status> {
        let (auth, req) = self.rate_limit(req).in_current_span().await?;
        auth.perm().super_user()?;

        req.get_or_insert(|req| async move {
            let mut announcement = Entity::write_by_id(req.announcement_id, &auth)?
                .columns([Column::Id, Column::ContestId])
                .one(self.db.deref())
                .instrument(info_span!("fetch").or_current())
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
            Ok(())
        })
        .await
        .with_grpc()
        .into()
    }
    #[instrument(
        skip_all,
        level = "info",
        name = "endpoint.Announcement.publish",
        err(level = "debug", Display)
    )]
    async fn publish(&self, req: Request<PublishRequest>) -> Result<Response<()>, Status> {
        let (auth, req) = self.rate_limit(req).in_current_span().await?;
        auth.perm().admin()?;

        req.get_or_insert(|req| async move {
            let mut model = Entity::find_by_id(req.id)
                .with_auth(&auth)
                .write()?
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
        name = "endpoint.Announcement.unpublish",
        err(level = "debug", Display)
    )]
    async fn unpublish(&self, req: Request<PublishRequest>) -> Result<Response<()>, Status> {
        let (auth, req) = self.rate_limit(req).in_current_span().await?;
        auth.perm().admin()?;

        req.get_or_insert(|req| async move {
            let mut model = Entity::find_by_id(req.id)
                .with_auth(&auth)
                .write()?
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
                .await?;
            Ok(())
        })
        .await
        .with_grpc()
        .into()
    }
}
