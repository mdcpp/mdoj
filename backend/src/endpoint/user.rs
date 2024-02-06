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
            // FIXME: capture Error instead!
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
        let (auth, req) = self.parse_request(req).await?;

        let (rev, size) = split_rev(req.size);
        let size = bound!(size, 64);
        let offset = bound!(req.offset(), 1024);

        let (pager, models) = match req.request.ok_or(Error::NotInPayload("request"))? {
            list_user_request::Request::Create(create) => {
                ColPaginator::new_fetch(
                    (create.sort_by(), Default::default()),
                    &auth,
                    size,
                    offset,
                    create.start_from_end,
                    &self.db,
                )
                .await
            }
            list_user_request::Request::Pager(old) => {
                let pager: ColPaginator = self.crypto.decode(old.session)?;
                pager.fetch(&auth, size, offset, rev, &self.db).await
            }
        }?;

        let next_session = self.crypto.encode(pager)?;
        let list = models.into_iter().map(|x| x.into()).collect();

        Ok(Response::new(ListUserResponse { list, next_session }))
    }
    #[instrument(skip_all, level = "debug")]
    async fn search_by_text(
        &self,
        req: Request<TextSearchRequest>,
    ) -> Result<Response<ListUserResponse>, Status> {
        let (auth, req) = self.parse_request(req).await?;

        let (rev, size) = split_rev(req.size);
        let size = bound!(size, 64);
        let offset = bound!(req.offset(), 1024);

        let (pager, models) = match req.request.ok_or(Error::NotInPayload("request"))? {
            text_search_request::Request::Text(text) => {
                TextPaginator::new_fetch(text, &auth, size, offset, true, &self.db).await
            }
            text_search_request::Request::Pager(old) => {
                let pager: TextPaginator = self.crypto.decode(old.session)?;
                pager.fetch(&auth, size, offset, rev, &self.db).await
            }
        }?;

        let next_session = self.crypto.encode(pager)?;
        let list = models.into_iter().map(|x| x.into()).collect();

        Ok(Response::new(ListUserResponse { list, next_session }))
    }
    async fn list_by_contest(
        &self,
        req: Request<ListByRequest>,
    ) -> Result<Response<ListUserResponse>, Status> {
        let (auth, req) = self.parse_request(req).await?;

        let (rev, size) = split_rev(req.size);
        let size = bound!(size, 64);
        let offset = bound!(req.offset(), 512);

        let (pager, models) = match req.request.ok_or(Error::NotInPayload("request"))? {
            list_by_request::Request::Create(create) => {
                ParentPaginator::new_fetch(
                    (0, create.parent_id),
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

        Ok(Response::new(ListUserResponse { list, next_session }))
    }
    #[instrument(skip_all, level = "debug")]
    async fn full_info(&self, _req: Request<UserId>) -> Result<Response<UserFullInfo>, Status> {
        Err(Status::cancelled("deprecated"))
    }
    #[instrument(skip_all, level = "debug")]
    async fn create(&self, req: Request<CreateUserRequest>) -> Result<Response<UserId>, Status> {
        let (auth, req) = self.parse_request(req).await?;
        let (user_id, perm) = auth.ok_or_default()?;

        check_length!(SHORT_ART_SIZE, req.info, username);

        if !perm.admin() {
            return Err(Error::RequirePermission(RoleLv::Admin).into());
        }

        let uuid = Uuid::parse_str(&req.request_id).map_err(Error::InvaildUUID)?;
        if let Some(x) = self.dup.check_i32(user_id, &uuid) {
            return Ok(Response::new(x.into()));
        };

        if !(perm.admin()) {
            return Err(Error::RequirePermission(RoleLv::Admin).into());
        }

        let mut model: ActiveModel = Default::default();

        tracing::debug!(username = req.info.username);

        let same_name = Entity::find()
            .filter(Column::Username.eq(req.info.username.clone()))
            .count(self.db.deref())
            .await
            .map_err(Into::<Error>::into)?;
        if same_name != 0 {
            return Err(Error::AlreadyExist("").into());
        }

        let hash = self.crypto.hash(req.info.password.as_str()).into();
        model.password = ActiveValue::set(hash);

        let new_perm: RoleLv = req.info.role().into();
        if !perm.root() && new_perm >= perm {
            return Err(Error::RequirePermission(new_perm).into());
        }

        fill_active_model!(model, req.info, username);

        model.permission = ActiveValue::set(new_perm as i32);

        let model = model
            .save(self.db.deref())
            .await
            .map_err(Into::<Error>::into)?;

        self.metrics.user.add(1, &[]);

        let id = model.id.unwrap();
        self.dup.store_i32(user_id, uuid, id);

        Ok(Response::new(id.into()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn update(&self, req: Request<UpdateUserRequest>) -> Result<Response<()>, Status> {
        let (auth, req) = self.parse_request(req).await?;

        let (user_id, perm) = auth.ok_or_default()?;

        let uuid = Uuid::parse_str(&req.request_id).map_err(Error::InvaildUUID)?;
        if self.dup.check_i32(user_id, &uuid).is_some() {
            return Ok(Response::new(()));
        };

        if !(perm.admin()) {
            return Err(Error::RequirePermission(RoleLv::Admin).into());
        }

        let mut model = Entity::write_filter(Entity::find_by_id(req.id), &auth)?
            .one(self.db.deref())
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB(Entity::DEBUG_NAME))?
            .into_active_model();

        if let Some(username) = req.info.username {
            model.username = ActiveValue::set(username);
        }
        if let Some(password) = req.info.password {
            let hash = self.crypto.hash(password.as_str()).into();
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

        let model = model
            .update(self.db.deref())
            .await
            .map_err(Into::<Error>::into)?;

        self.dup.store_i32(user_id, uuid, model.id);

        Ok(Response::new(()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn remove(&self, req: Request<UserId>) -> Result<Response<()>, Status> {
        let (auth, req) = self.parse_request(req).await?;

        let result = Entity::write_filter(Entity::delete_by_id(Into::<i32>::into(req.id)), &auth)?
            .exec(self.db.deref())
            .await
            .map_err(Into::<Error>::into)?;

        if result.rows_affected == 0 {
            return Err(Error::NotInDB(Entity::DEBUG_NAME).into());
        }

        self.metrics.user.add(-1, &[]);

        self.token.remove_by_user_id(req.id).await?;

        Ok(Response::new(()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn update_password(
        &self,
        req: Request<UpdatePasswordRequest>,
    ) -> Result<Response<()>, Status> {
        let (auth, req) = self.parse_request(req).await?;
        let (user_id, _) = auth.ok_or_default()?;

        let model = user::Entity::find()
            .filter(user::Column::Username.eq(req.username))
            .one(self.db.deref())
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB(Entity::DEBUG_NAME))?;

        if !self.crypto.hash_eq(req.password.as_str(), &model.password) {
            return Err(Error::PermissionDeny("wrong original password").into());
        }

        let mut model = model.into_active_model();
        model.password = ActiveValue::Set(self.crypto.hash(&req.new_password).into());

        model
            .update(self.db.deref())
            .await
            .map_err(Into::<Error>::into)?;

        Entity::delete_by_id(user_id)
            .exec(self.db.deref())
            .await
            .map_err(Into::<Error>::into)?;

        self.token.remove_by_user_id(user_id).await?;

        return Ok(Response::new(()));
    }

    #[instrument(skip_all, level = "debug")]
    async fn my_info(&self, req: Request<()>) -> Result<Response<UserInfo>, Status> {
        let (auth, _req) = self.parse_request(req).await?;
        let (user_id, _) = auth.ok_or_default()?;

        let model = Entity::find_by_id(user_id)
            .one(self.db.deref())
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::Unreachable(
                "token should be deleted before user can request its info",
            ))?;

        Ok(Response::new(model.into()))
    }
}
