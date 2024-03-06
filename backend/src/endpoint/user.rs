use super::tools::*;

use crate::grpc::backend::user_set_server::*;
use crate::grpc::backend::*;

use crate::entity::user;
use crate::entity::user::*;

impl From<i32> for UserId {
    fn from(value: i32) -> Self {
        Self { id: value }
    }
}

impl From<UserId> for i32 {
    fn from(value: UserId) -> Self {
        value.id
    }
}

impl From<Model> for UserInfo {
    fn from(value: Model) -> Self {
        UserInfo {
            username: value.username,
            // FIXME: capture Error(database corruption?) instead!
            score: value.score.try_into().unwrap_or_default(),
            id: value.id.into(),
        }
    }
}

#[async_trait]
impl UserSet for Arc<Server> {
    #[instrument(skip_all, level = "debug")]
    async fn list(
        &self,
        req: Request<ListUserRequest>,
    ) -> Result<Response<ListUserResponse>, Status> {
        let (auth, rev, size, offset, pager) = parse_pager_param!(self, req);

        let (pager, models) = match pager {
            list_user_request::Request::Create(create) => {
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
            list_user_request::Request::Pager(old) => {
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

        Ok(Response::new(ListUserResponse {
            list,
            next_session,
            remain,
        }))
    }
    #[instrument(skip_all, level = "debug")]
    async fn search_by_text(
        &self,
        req: Request<TextSearchRequest>,
    ) -> Result<Response<ListUserResponse>, Status> {
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

        Ok(Response::new(ListUserResponse {
            list,
            next_session,
            remain,
        }))
    }
    async fn list_by_contest(
        &self,
        req: Request<ListByRequest>,
    ) -> Result<Response<ListUserResponse>, Status> {
        let (auth, rev, size, offset, pager) = parse_pager_param!(self, req);

        let (pager, models) = match pager {
            list_by_request::Request::Create(create) => {
                ParentPaginator::new_fetch(
                    (0, create.parent_id),
                    &auth,
                    size,
                    offset,
                    create.start_from_end(),
                    &self.db,
                )
                .in_current_span()
                .await
            }
            list_by_request::Request::Pager(old) => {
                let span = tracing::info_span!("paginate").or_current();
                let pager: ParentPaginator = span.in_scope(|| self.crypto.decode(old.session))?;
                pager
                    .fetch(&auth, size, offset, rev, &self.db)
                    .instrument(span)
                    .await
            }
        }?;

        let remain = pager.remain(&auth, &self.db).in_current_span().await?;

        let next_session = self.crypto.encode(pager)?;
        let list = models.into_iter().map(|x| x.into()).collect();

        Ok(Response::new(ListUserResponse {
            list,
            next_session,
            remain,
        }))
    }
    #[instrument(skip_all, level = "debug")]
    async fn full_info(&self, _req: Request<UserId>) -> Result<Response<UserFullInfo>, Status> {
        Err(Status::cancelled("deprecated"))
    }
    #[instrument(skip_all, level = "debug")]
    async fn create(&self, req: Request<CreateUserRequest>) -> Result<Response<UserId>, Status> {
        let (auth, req) = self
            .parse_request_n(req, NonZeroU32!(5))
            .in_current_span()
            .await?;
        let (user_id, perm) = auth.ok_or_default()?;

        check_length!(SHORT_ART_SIZE, req.info, username);

        if !perm.admin() {
            return Err(Error::RequirePermission(RoleLv::Admin).into());
        }

        let uuid = Uuid::parse_str(&req.request_id).map_err(Error::InvaildUUID)?;
        if let Some(x) = self.dup.check::<UserId>(user_id, uuid) {
            return Ok(Response::new(x));
        };
        if !(perm.admin()) {
            return Err(Error::RequirePermission(RoleLv::Admin).into());
        }

        let mut model: ActiveModel = Default::default();

        tracing::debug!(username = req.info.username);

        let hash = self.crypto.hash(req.info.password.as_str());
        let new_perm: RoleLv = req.info.role().into();
        if !perm.root() && new_perm >= perm {
            return Err(Error::RequirePermission(new_perm).into());
        }

        let txn = self.db.begin().await.map_err(Error::DBErr)?;

        let same_name = Entity::find()
            .filter(Column::Username.eq(req.info.username.as_str()))
            .count(&txn)
            .instrument(info_span!("check_exist").or_current())
            .await
            .map_err(Into::<Error>::into)?;
        if same_name != 0 {
            return Err(Error::AlreadyExist(req.info.username).into());
        }

        model.password = ActiveValue::set(hash);
        model.permission = ActiveValue::set(new_perm as i32);
        fill_active_model!(model, req.info, username);

        let model = model
            .save(&txn)
            .instrument(info_span!("save").or_current())
            .await
            .map_err(Into::<Error>::into)?;

        let id: UserId = model.id.clone().unwrap().into();
        self.dup.store(user_id, uuid, id.clone());

        txn.commit()
            .await
            .map_err(|_| Error::AlreadyExist(model.username.as_ref().to_string()))?;

        tracing::debug!(id = id.id, "user_created");
        self.metrics.user(1);

        Ok(Response::new(id))
    }
    #[instrument(skip_all, level = "debug")]
    async fn update(&self, req: Request<UpdateUserRequest>) -> Result<Response<()>, Status> {
        let (auth, req) = self
            .parse_request_n(req, NonZeroU32!(5))
            .in_current_span()
            .await?;

        let (user_id, perm) = auth.ok_or_default()?;

        let uuid = Uuid::parse_str(&req.request_id).map_err(Error::InvaildUUID)?;
        if let Some(x) = self.dup.check::<()>(user_id, uuid) {
            return Ok(Response::new(x));
        };
        if !(perm.admin()) {
            return Err(Error::RequirePermission(RoleLv::Admin).into());
        }

        let mut model = Entity::write_filter(Entity::find_by_id(req.id), &auth)?
            .one(self.db.deref())
            .instrument(info_span!("fetch").or_current())
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB)?
            .into_active_model();

        if let Some(username) = req.info.username {
            model.username = ActiveValue::set(username);
        }
        if let Some(password) = req.info.password {
            let hash = self.crypto.hash(password.as_str());
            model.password = ActiveValue::set(hash);
        }
        if let Some(new_perm) = req.info.role {
            let new_perm: Role = new_perm.try_into().unwrap_or_default();
            let new_perm: RoleLv = new_perm.into();
            if !perm.admin() {
                return Err(Error::RequirePermission(RoleLv::Admin).into());
            }
            if !perm.root() && new_perm > perm {
                return Err(Error::RequirePermission(RoleLv::Root).into());
            }
            model.permission = ActiveValue::set(new_perm as i32);
            if new_perm > perm {
                return Err(Error::RequirePermission(new_perm).into());
            }
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
    async fn remove(&self, req: Request<UserId>) -> Result<Response<()>, Status> {
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

        self.metrics.user(-1);

        self.token.remove_by_user_id(req.id).await?;

        Ok(Response::new(()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn update_password(
        &self,
        req: Request<UpdatePasswordRequest>,
    ) -> Result<Response<()>, Status> {
        let (auth, req) = self
            .parse_request_n(req, NonZeroU32!(5))
            .in_current_span()
            .await?;
        let (user_id, _) = auth.ok_or_default()?;

        let model = user::Entity::find()
            .filter(user::Column::Username.eq(req.username))
            .one(self.db.deref())
            .instrument(info_span!("fetch").or_current())
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB)?;

        if !self.crypto.hash_eq(req.password.as_str(), &model.password) {
            return Err(Error::PermissionDeny("wrong original password").into());
        }

        let mut model = model.into_active_model();
        model.password = ActiveValue::Set(self.crypto.hash(&req.new_password));

        model
            .update(self.db.deref())
            .instrument(info_span!("update").or_current())
            .await
            .map_err(atomic_fail)?;

        self.token.remove_by_user_id(user_id).await?;

        return Ok(Response::new(()));
    }

    #[instrument(skip_all, level = "debug")]
    async fn my_info(&self, req: Request<()>) -> Result<Response<UserInfo>, Status> {
        let (auth, _) = self
            .parse_request_n(req, NonZeroU32!(5))
            .in_current_span()
            .await?;
        let (user_id, _) = auth.ok_or_default()?;

        let model = Entity::find_by_id(user_id)
            .one(self.db.deref())
            .instrument(info_span!("fetch").or_current())
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::Unreachable(
                "token should be deleted before user can request its info after user deletion",
            ))?;

        Ok(Response::new(model.into()))
    }
}
