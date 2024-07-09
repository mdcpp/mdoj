use super::tools::*;

use grpc::backend::user_service_server::*;
use grpc::backend::*;

use crate::entity::user;
use crate::entity::user::*;

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
impl UserService for ArcServer {
    #[instrument(skip_all, level = "debug")]
    async fn list(
        &self,
        req: Request<ListUserRequest>,
    ) -> Result<Response<ListUserResponse>, Status> {
        todo!()
    }
    #[instrument(skip_all, level = "debug")]
    async fn full_info(&self, _req: Request<Id>) -> Result<Response<UserFullInfo>, Status> {
        Err(Status::cancelled("deprecated"))
    }
    #[instrument(skip_all, level = "debug")]
    async fn create(&self, req: Request<CreateUserRequest>) -> Result<Response<Id>, Status> {
        let (auth, req) = self
            .parse_request_n(req, NonZeroU32!(5))
            .in_current_span()
            .await?;

        check_length!(SHORT_ART_SIZE, req.info, username);

        let uuid = Uuid::parse_str(&req.request_id).map_err(Error::InvaildUUID)?;

        let (user_id, perm) = auth.ok_or_default()?;

        if let Some(x) = self.dup.check::<Id>(user_id, uuid) {
            return Ok(Response::new(x));
        };

        let new_perm: RoleLv = req.info.role().into();

        if (perm as i32)
            > self
                .config
                .default_role
                .clone()
                .map(|x| x as i32)
                .unwrap_or_default()
        {
            if !perm.admin() {
                return Err(Error::RequirePermission(RoleLv::Admin).into());
            } else if !perm.root() && new_perm >= perm {
                return Err(Error::RequirePermission(new_perm).into());
            }
        }

        let mut model: ActiveModel = Default::default();

        tracing::debug!(username = req.info.username);

        let hash = self.crypto.hash(req.info.password.as_str());
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

        let id: Id = model.id.clone().unwrap().into();
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
