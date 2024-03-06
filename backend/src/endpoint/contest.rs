use super::tools::*;

use crate::entity::{contest::*, *};
use crate::grpc::backend::contest_set_server::*;
use crate::grpc::backend::*;
use crate::grpc::into_chrono;
use crate::grpc::into_prost;

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

impl From<PartialModel> for ContestInfo {
    fn from(value: PartialModel) -> Self {
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
        req: Request<ListContestRequest>,
    ) -> Result<Response<ListContestResponse>, Status> {
        let (auth, rev, size, offset, pager) = parse_pager_param!(self, req);

        let (pager, models) = match pager {
            list_contest_request::Request::Create(create) => {
                ColPaginator::new_fetch(
                    (create.sort_by(), Default::default()),
                    &auth,
                    size,
                    offset,
                    create.start_from_end(),
                    &self.db,
                )
                .in_current_span()
                .await
            }
            list_contest_request::Request::Pager(old) => {
                let span = tracing::info_span!("paginate").or_current();
                let pager: ColPaginator = span.in_scope(|| self.crypto.decode(old.session))?;
                pager
                    .fetch(&auth, size, offset, rev, &self.db)
                    .instrument(span)
                    .await
            }
        }?;

        let remain = pager.remain(&auth, &self.db).in_current_span().await?;

        let next_session = self.crypto.encode(pager)?;
        let list = models.into_iter().map(|x| x.into()).collect();

        Ok(Response::new(ListContestResponse {
            list,
            next_session,
            remain,
        }))
    }
    #[instrument(skip_all, level = "debug")]
    async fn search_by_text(
        &self,
        req: Request<TextSearchRequest>,
    ) -> Result<Response<ListContestResponse>, Status> {
        let (auth, rev, size, offset, pager) = parse_pager_param!(self, req);

        let (pager, models) = match pager {
            text_search_request::Request::Text(text) => {
                TextPaginator::new_fetch(text, &auth, size, offset, true, &self.db)
                    .in_current_span()
                    .await
            }
            text_search_request::Request::Pager(old) => {
                let span = tracing::info_span!("paginate").or_current();
                let pager: TextPaginator = span.in_scope(|| self.crypto.decode(old.session))?;
                pager
                    .fetch(&auth, size, offset, rev, &self.db)
                    .instrument(span)
                    .await
            }
        }?;

        let remain = pager.remain(&auth, &self.db).in_current_span().await?;

        let next_session = self.crypto.encode(pager)?;
        let list = models.into_iter().map(|x| x.into()).collect();

        Ok(Response::new(ListContestResponse {
            list,
            next_session,
            remain,
        }))
    }
    #[instrument(skip_all, level = "debug")]
    async fn full_info(
        &self,
        req: Request<ContestId>,
    ) -> Result<Response<ContestFullInfo>, Status> {
        let (_, req) = self
            .parse_request_n(req, NonZeroU32!(5))
            .in_current_span()
            .await?;

        let query = Entity::find_by_id::<i32>(req.into()).filter(Column::Public.eq(true));
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
        req: Request<CreateContestRequest>,
    ) -> Result<Response<ContestId>, Status> {
        let (auth, req) = self
            .parse_request_n(req, NonZeroU32!(5))
            .in_current_span()
            .await?;
        let (user_id, perm) = auth.ok_or_default()?;

        check_length!(SHORT_ART_SIZE, req.info, title, tags);
        check_length!(LONG_ART_SIZE, req.info, content);

        let uuid = Uuid::parse_str(&req.request_id).map_err(Error::InvaildUUID)?;
        if let Some(x) = self.dup.check::<ContestId>(user_id, uuid) {
            return Ok(Response::new(x));
        };

        if !perm.super_user() {
            return Err(Error::RequirePermission(RoleLv::Super).into());
        }

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

        let id: ContestId = model.id.clone().unwrap().into();
        self.dup.store(user_id, uuid, id.clone());

        tracing::debug!(id = id.id, "contest_created");
        self.metrics.contest(1);

        Ok(Response::new(id))
    }
    #[instrument(skip_all, level = "debug")]
    async fn update(&self, req: Request<UpdateContestRequest>) -> Result<Response<()>, Status> {
        let (auth, req) = self
            .parse_request_n(req, NonZeroU32!(5))
            .in_current_span()
            .await?;
        let (user_id, perm) = auth.ok_or_default()?;

        check_exist_length!(SHORT_ART_SIZE, req.info, title, tags);
        check_exist_length!(LONG_ART_SIZE, req.info, content);

        let uuid = Uuid::parse_str(&req.request_id).map_err(Error::InvaildUUID)?;
        if let Some(x) = self.dup.check::<()>(user_id, uuid) {
            return Ok(Response::new(x));
        };
        if !perm.super_user() {
            return Err(Error::RequirePermission(RoleLv::Super).into());
        }

        let mut model = Entity::write_filter(Entity::find_by_id(req.id), &auth)?
            .one(self.db.deref())
            .instrument(info_span!("fetch").or_current())
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB)?;

        check_exist_length!(SHORT_ART_SIZE, req.info, password);
        if let Some(src) = req.info.password {
            if let Some(tar) = model.password.as_ref() {
                if perm.root() || self.crypto.hash_eq(&src, tar) {
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
            .map_err(atomic_fail)?;

        self.dup.store(user_id, uuid, ());

        Ok(Response::new(()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn remove(&self, req: Request<ContestId>) -> Result<Response<()>, Status> {
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

        self.metrics.contest(-1);
        tracing::debug!(id = req.id, "contest_remove");

        Ok(Response::new(()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn join(&self, req: Request<JoinContestRequest>) -> Result<Response<()>, Status> {
        let (auth, req) = self
            .parse_request_n(req, NonZeroU32!(5))
            .in_current_span()
            .await?;
        let (user_id, perm) = auth.ok_or_default()?;

        let model = Entity::read_filter(Entity::find_by_id(req.id.id), &auth)?
            .one(self.db.deref())
            .instrument(info_span!("fetch").or_current())
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB)?;

        let empty_password = "".to_string();
        if let Some(tar) = model.password {
            if (!perm.root())
                && (!self
                    .crypto
                    .hash_eq(req.password.as_ref().unwrap_or(&empty_password), &tar))
                && model.public
            {
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

        tracing::debug!(user_id = user_id, contest_id = req.id.id);

        Ok(Response::new(()))
    }
}
