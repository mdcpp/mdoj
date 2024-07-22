use super::tools::*;

use crate::entity::{
    contest::{Paginator, *},
    *,
};

use grpc::backend::contest_server::*;

impl<'a> From<WithAuth<'a, Model>> for ContestFullInfo {
    fn from(value: WithAuth<'a, Model>) -> Self {
        let model = value.1;
        let writable = Entity::writable(&model, value.0);
        ContestFullInfo {
            info: model.clone().into(),
            content: model.content,
            host: model.hoster,
            writable,
        }
    }
}

impl WithAuthTrait for Model {}

impl From<user_contest::Model> for UserRank {
    fn from(value: user_contest::Model) -> Self {
        UserRank {
            user_id: value.user_id,
            score: value.score,
        }
    }
}

impl From<Model> for ContestInfo {
    fn from(value: Model) -> Self {
        ContestInfo {
            id: value.id,
            title: value.title,
            begin: into_prost(value.begin),
            end: into_prost(value.end),
            need_password: value.password.is_some(),
        }
    }
}

impl From<PartialModel> for ContestInfo {
    fn from(value: PartialModel) -> Self {
        ContestInfo {
            id: value.id,
            title: value.title,
            begin: into_prost(value.begin),
            end: into_prost(value.end),
            need_password: value.password.is_some(),
        }
    }
}

#[async_trait]
impl Contest for ArcServer {
    #[instrument(
        skip_all,
        level = "info",
        name = "endpoint.Contest.list",
        err(level = "debug", Display)
    )]
    async fn list(
        &self,
        req: Request<ListContestRequest>,
    ) -> Result<Response<ListContestResponse>, Status> {
        let (auth, req) = self.rate_limit(req).in_current_span().await?;
        req.bound_check()?;

        let paginator = match req.request.ok_or(Error::NotInPayload("request"))? {
            list_contest_request::Request::Create(create) => {
                let query = create.query.unwrap_or_default();
                let start_from_end = create.order == Order::Descend as i32;
                if let Some(text) = query.text {
                    Paginator::new_text(text, start_from_end)
                } else if let Some(sort) = query.sort_by {
                    Paginator::new_sort(sort.try_into().unwrap_or_default(), start_from_end)
                } else {
                    Paginator::new(start_from_end)
                }
            }
            list_contest_request::Request::Paginator(x) => self.crypto.decode(x)?,
        };
        let mut paginator = paginator.with_auth(&auth).with_db(&self.db);

        let list = paginator
            .fetch(req.size, req.offset)
            .in_current_span()
            .await?;
        let remain = paginator.remain().in_current_span().await?;

        let paginator = paginator.into_inner();

        Ok(Response::new(ListContestResponse {
            list: list.into_iter().map(Into::into).collect(),
            paginator: self.crypto.encode(paginator)?,
            remain,
        }))
    }
    #[instrument(
        skip_all,
        level = "info",
        name = "endpoint.Contest.full_info",
        err(level = "debug", Display)
    )]
    async fn full_info(&self, req: Request<Id>) -> Result<Response<ContestFullInfo>, Status> {
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
        name = "endpoint.Contest.create",
        err(level = "debug", Display)
    )]
    async fn create(&self, req: Request<CreateContestRequest>) -> Result<Response<Id>, Status> {
        let (auth, req) = self.rate_limit(req).in_current_span().await?;
        req.bound_check()?;

        let (user_id, perm) = auth.assume_login()?;
        perm.super_user()?;

        req.get_or_insert(|req| async {
            let mut model: ActiveModel = Default::default();
            model.hoster = ActiveValue::Set(user_id);

            fill_active_model!(model, req.info, title, content, tags);

            let password: Vec<u8> = req
                .info
                .password
                .map(|a| self.crypto.hash(&a))
                .ok_or(Error::NotInPayload("password"))?;
            model.password = ActiveValue::Set(Some(password));

            model.begin = ActiveValue::Set(into_chrono(req.info.begin));
            model.end = ActiveValue::Set(into_chrono(req.info.end));

            let model = model
                .save(self.db.deref())
                .instrument(info_span!("save").or_current())
                .await
                .map_err(Into::<Error>::into)?;

            let model = model
                .save(self.db.deref())
                .instrument(info_span!("save").or_current())
                .await
                .map_err(Into::<Error>::into)?;

            let id = *model.id.as_ref();

            tracing::info!(count.contest = 1, id = id);

            Ok(id.into())
        })
        .await
        .with_grpc()
        .into()
    }
    #[instrument(
        skip_all,
        level = "info",
        name = "endpoint.Contest.update",
        err(level = "debug", Display)
    )]
    async fn update(&self, req: Request<UpdateContestRequest>) -> Result<Response<()>, Status> {
        let (auth, req) = self.rate_limit(req).in_current_span().await?;
        req.bound_check()?;

        let (_, perm) = auth.assume_login()?;

        req.get_or_insert(|req| async move {
            tracing::trace!(id = req.id);

            let mut model = Entity::find_by_id(req.id).with_auth(&auth).write()?
            .one(self.db.deref())
            .instrument(info_span!("fetch").or_current())
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB)?;

            if let Some(src) = req.info.password {
                if let Some(tar) = model.password.as_ref() {
                    if perm.root().is_ok() || self.crypto.hash_eq(&src, tar) {
                        let hash = self.crypto.hash(&src);
                        model.password = Some(hash);
                    } else {
                        return Err(Error::PermissionDeny(
                            "mismatch password(root can update password without entering original password)",
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

            model
                .update(self.db.deref())
                .instrument(info_span!("update").or_current())
                .await
                ?;
                Ok(())
        })
        .await
        .with_grpc()
        .into()
    }
    #[instrument(
        skip_all,
        level = "info",
        name = "endpoint.Contest.remove",
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
                tracing::info!(counter.contest = -1, id = req.id);
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
        name = "endpoint.Contest.join",
        err(level = "debug", Display)
    )]
    async fn join(&self, req: Request<JoinContestRequest>) -> Result<Response<()>, Status> {
        let (auth, req) = self.rate_limit(req).in_current_span().await?;
        let (user_id, _) = auth.assume_login()?;

        req.get_or_insert(|req| async move {
            let model = Entity::find_by_id(req.id)
                .with_auth(&auth)
                .read()?
                .one(self.db.deref())
                .instrument(info_span!("fetch").or_current())
                .await
                .map_err(Into::<Error>::into)?
                .ok_or(Error::NotInDB)?;
            // FIXME: abstract away password checking logic

            if let Some(tar) = model.password {
                let password = req
                    .password
                    .as_ref()
                    .ok_or(Error::NotInPayload("password"))?;
                if !self.crypto.hash_eq(&password, &tar) {
                    return Err(Error::PermissionDeny("mismatched password").into());
                }
            }

            let pivot = user_contest::ActiveModel {
                user_id: ActiveValue::Set(user_id),
                contest_id: ActiveValue::Set(model.id),
                ..Default::default()
            };

            pivot
                .save(self.db.deref())
                .instrument(info_span!("insert_pviot").or_current())
                .await
                .map_err(Into::<Error>::into)?;

            tracing::debug!(user_id = user_id, contest_id = req.id);
            Ok(())
        })
        .await
        .with_grpc()
        .into()
    }
    #[instrument(
        skip_all,
        level = "info",
        name = "endpoint.Contest.publish",
        err(level = "debug", Display)
    )]
    async fn publish(&self, req: Request<PublishRequest>) -> Result<Response<()>, Status> {
        let (auth, req) = self.rate_limit(req).in_current_span().await?;

        req.get_or_insert(|req| async move {
            let mut model = Entity::find_by_id(req.id)
                .with_auth(&auth)
                .write()?
                .columns([Column::Id])
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
        name = "endpoint.Contest.unpublish",
        err(level = "debug", Display)
    )]
    async fn unpublish(&self, req: Request<PublishRequest>) -> Result<Response<()>, Status> {
        let (auth, req) = self.rate_limit(req).in_current_span().await?;

        req.get_or_insert(|req| async move {
            let mut model = Entity::find_by_id(req.id)
                .with_auth(&auth)
                .write()?
                .columns([Column::Id])
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
