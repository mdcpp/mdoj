use super::tools::*;

use crate::controller::token::UserPermBytes;
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
            score: value.score,
            id: value.id.into(),
        }
    }
}

impl From<UserPermBytes> for Permission {
    fn from(value: UserPermBytes) -> Self {
        Permission { flags: value.0 }
    }
}

impl From<Permission> for UserPermBytes {
    fn from(value: Permission) -> Self {
        UserPermBytes(value.flags)
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
        let size = req.size;
        let offset = req.offset();

        let (pager, models) = match req.request.ok_or(Error::NotInPayload("request"))? {
            list_user_request::Request::Create(create) => {
                ColPaginator::new_fetch(
                    (create.sort_by(), Default::default()),
                    &auth,
                    size,
                    offset,
                    create.reverse,
                )
                .await
            }
            list_user_request::Request::Pager(old) => {
                let pager: ColPaginator = self.crypto.decode(old.session)?;
                pager.fetch(&auth, size, offset, old.reverse).await
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
        let size = req.size;
        let offset = req.offset();

        let (pager, models) = match req.request.ok_or(Error::NotInPayload("request"))? {
            text_search_request::Request::Text(text) => {
                TextPaginator::new_fetch(text, &auth, size, offset, true).await
            }
            text_search_request::Request::Pager(old) => {
                let pager: TextPaginator = self.crypto.decode(old.session)?;
                pager.fetch(&auth, size, offset, old.reverse).await
            }
        }?;

        let next_session = self.crypto.encode(pager)?;
        let list = models.into_iter().map(|x| x.into()).collect();

        Ok(Response::new(ListUserResponse { list, next_session }))
    }
    #[instrument(skip_all, level = "debug")]
    async fn full_info(&self, req: Request<UserId>) -> Result<Response<UserFullInfo>, Status> {
        let (auth, _) = self.parse_request(req).await?;

        if !auth.is_root() {
            return Err(Error::Unauthenticated.into());
        }

        Err(Status::cancelled("deprecated"))
    }
    #[instrument(skip_all, level = "debug")]
    async fn create(&self, req: Request<CreateUserRequest>) -> Result<Response<UserId>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;
        let (user_id, perm) = auth.ok_or_default()?;

        let uuid = Uuid::parse_str(&req.request_id).map_err(Error::InvaildUUID)?;
        if let Some(x) = self.dup.check_i32(user_id, &uuid) {
            return Ok(Response::new(x.into()));
        };

        if !(perm.can_root() || perm.can_manage_user()) {
            return Err(Error::PermissionDeny("Can't create user").into());
        }

        let mut model: ActiveModel = Default::default();

        tracing::debug!(username = req.info.username);

        let same_name = Entity::find()
            .filter(Column::Username.eq(req.info.username.clone()))
            .count(db)
            .await
            .map_err(Into::<Error>::into)?;
        if same_name != 0 {
            return Err(Error::AlreadyExist("").into());
        }

        fill_active_model!(model, req.info, username);

        let hash = self.crypto.hash(req.info.password.as_str()).into();
        model.password = ActiveValue::set(hash);
        let new_perm = Into::<UserPermBytes>::into(req.info.permission);

        if new_perm != UserPermBytes::default() && !perm.can_root() {
            return Err(Error::PermissionDeny("Can't set permission").into());
        }
        model.permission = ActiveValue::set(new_perm.0);

        let model = model.save(db).await.map_err(Into::<Error>::into)?;

        self.metrics.user.add(1, &[]);

        let id = model.id.unwrap();
        self.dup.store_i32(user_id, uuid, id);

        Ok(Response::new(id.into()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn update(&self, req: Request<UpdateUserRequest>) -> Result<Response<()>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;

        let (user_id, perm) = auth.ok_or_default()?;

        let uuid = Uuid::parse_str(&req.request_id).map_err(Error::InvaildUUID)?;
        if self.dup.check_i32(user_id, &uuid).is_some() {
            return Ok(Response::new(()));
        };

        if !(perm.can_root() || perm.can_manage_user()) {
            return Err(Error::RequirePermission(Entity::DEBUG_NAME).into());
        }

        let mut model = Entity::write_filter(Entity::find_by_id(req.id), &auth)?
            .one(db)
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
        if let Some(permission) = req.info.permission {
            if !perm.can_root() {
                return Err(Error::PermissionDeny("Can't set permission").into());
            }
            model.permission = ActiveValue::set(Into::<UserPermBytes>::into(permission).0);
        }

        let model = model.update(db).await.map_err(Into::<Error>::into)?;

        self.dup.store_i32(user_id, uuid, model.id);

        Ok(Response::new(()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn remove(&self, req: Request<UserId>) -> Result<Response<()>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;

        let result = Entity::write_filter(Entity::delete_by_id(Into::<i32>::into(req.id)), &auth)?
            .exec(db)
            .await
            .map_err(Into::<Error>::into)?;

        if result.rows_affected == 0 {
            return Err(Error::NotInDB(Entity::DEBUG_NAME).into());
        }

        self.metrics.user.add(-1, &[]);

        // TODO: remove user's token

        self.token.remove_by_user_id(req.id).await?;

        Ok(Response::new(()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn update_password(
        &self,
        req: Request<UpdatePasswordRequest>,
    ) -> Result<Response<()>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;
        let (user_id, _) = auth.ok_or_default()?;

        let model = user::Entity::find()
            .filter(user::Column::Username.eq(req.username))
            .one(db)
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB(Entity::DEBUG_NAME))?;

        if !self.crypto.hash_eq(req.password.as_str(), &model.password) {
            return Err(Error::PermissionDeny("password").into());
        }

        let mut model = model.into_active_model();
        model.password = ActiveValue::Set(self.crypto.hash(&req.new_password).into());

        model.update(db).await.map_err(Into::<Error>::into)?;

        Entity::delete_by_id(user_id)
            .exec(db)
            .await
            .map_err(Into::<Error>::into)?;

        self.token.remove_by_user_id(user_id).await?;

        return Ok(Response::new(()));
    }

    #[instrument(skip_all, level = "debug")]
    async fn my_info(&self, req: Request<()>) -> Result<Response<UserInfo>, Status> {
        let db = DB.get().unwrap();
        let (auth, _req) = self.parse_request(req).await?;
        let (user_id, _) = auth.ok_or_default()?;

        let model = Entity::find_by_id(user_id)
            .one(db)
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::Unreachable(
                "token should be deleted before user can request its info",
            ))?;

        Ok(Response::new(model.into()))
    }
}
