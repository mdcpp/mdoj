use super::endpoints::*;
use super::tools::*;

use crate::controller::token::UserPermBytes;
use crate::grpc::backend::user_set_server::*;
use crate::grpc::backend::*;

use entity::user;
use entity::user::*;

impl Filter for Entity {
    fn read_filter<S: QueryFilter + Send>(query: S, _: &Auth) -> Result<S, Error> {
        Ok(query)
    }

    fn write_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
        let (user_id, perm) = auth.ok_or_default()?;
        if perm.can_root() || perm.can_manage_user() {
            return Ok(query);
        }
        Ok(query.filter(Column::Id.eq(user_id)))
    }
}

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
        Permission {
            can_manage_contest: value.can_manage_contest(),
            can_manage_problem: value.can_manage_problem(),
            can_manage_announcement: value.can_manage_announcement(),
            can_manage_education: value.can_manage_education(),
            can_manage_user: value.can_manage_user(),
            can_root: value.can_root(),
            can_link: value.can_link(),
            can_publish: value.can_publish(),
        }
    }
}

impl From<Permission> for UserPermBytes {
    fn from(value: Permission) -> Self {
        let mut perm = UserPermBytes::default();

        if value.can_manage_contest {
            perm.grant_manage_contest(true);
        }
        if value.can_manage_problem {
            perm.grant_manage_problem(true);
        }
        if value.can_manage_announcement {
            perm.grant_manage_announcement(true);
        }
        if value.can_manage_education {
            perm.grant_manage_education(true);
        }
        if value.can_manage_user {
            perm.grant_manage_user(true);
        }
        if value.can_root {
            perm.grant_root(true);
        }
        if value.can_link {
            perm.grant_link(true);
        }
        if value.can_publish {
            perm.grant_publish(true);
        }

        perm
    }
}

#[async_trait]
impl UserSet for Arc<Server> {
    #[instrument(skip_all, level = "debug")]
    async fn list(&self, req: Request<ListRequest>) -> Result<Response<ListUserResponse>, Status> {
        let (auth, req) = self.parse_request(req).await?;

        let mut reverse = false;
        let mut pager: Pager<Entity> = match req.request.ok_or(Error::NotInPayload("request"))? {
            list_request::Request::Create(create) => {
                Pager::sort_search(create.sort_by(), create.reverse)
            }
            list_request::Request::Pager(old) => {
                reverse = old.reverse;
                <Pager<Entity> as NoParentPager<Entity>>::from_raw(old.session)?
            }
        };

        let list = pager
            .fetch(req.size, req.offset.unwrap_or_default(), reverse, &auth)
            .await?
            .into_iter()
            .map(|x| x.into())
            .collect();

        let next_session = pager.into_raw();

        Ok(Response::new(ListUserResponse { list, next_session }))
    }
    #[instrument(skip_all, level = "debug")]
    async fn search_by_text(
        &self,
        req: Request<TextSearchRequest>,
    ) -> Result<Response<ListUserResponse>, Status> {
        let (auth, req) = self.parse_request(req).await?;

        let mut reverse = false;
        let mut pager: Pager<Entity> = match req.request.ok_or(Error::NotInPayload("request"))? {
            text_search_request::Request::Text(create) => Pager::text_search(create),
            text_search_request::Request::Pager(old) => {
                reverse = old.reverse;
                <Pager<_> as NoParentPager<Entity>>::from_raw(old.session)?
            }
        };

        let list = pager
            .fetch(req.size, req.offset.unwrap_or_default(), reverse, &auth)
            .await?
            .into_iter()
            .map(|x| x.into())
            .collect();

        let next_session = pager.into_raw();

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
        if let Some(x) = self.dup.check(user_id, &uuid) {
            return Ok(Response::new(x.into()));
        };

        if !(perm.can_root() || perm.can_manage_user()) {
            return Err(Error::PremissionDeny("Can't create user").into());
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
            return Err(Error::PremissionDeny("Can't set permission").into());
        }
        model.permission = ActiveValue::set(new_perm.0);

        let model = model.save(db).await.map_err(Into::<Error>::into)?;

        self.dup.store(user_id, uuid, model.id.clone().unwrap());

        Ok(Response::new(model.id.unwrap().into()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn update(&self, req: Request<UpdateUserRequest>) -> Result<Response<()>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;

        let (user_id, perm) = auth.ok_or_default()?;

        let uuid = Uuid::parse_str(&req.request_id).map_err(Error::InvaildUUID)?;
        if self.dup.check(user_id, &uuid).is_some() {
            return Ok(Response::new(()));
        };

        if !(perm.can_root() || perm.can_manage_problem()) {
            return Err(Error::PremissionDeny("Can't update user").into());
        }

        let mut model = Entity::write_filter(Entity::find_by_id(req.id), &auth)?
            .one(db)
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB("user"))?
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
                return Err(Error::PremissionDeny("Can't set permission").into());
            }
            model.permission = ActiveValue::set(Into::<UserPermBytes>::into(permission).0);
        }

        let model = model.update(db).await.map_err(Into::<Error>::into)?;

        self.dup.store(user_id, uuid, model.id);

        Ok(Response::new(()))
    }
    #[instrument(skip_all, level = "debug")]
    async fn remove(&self, req: Request<UserId>) -> Result<Response<()>, Status> {
        let db = DB.get().unwrap();
        let (auth, req) = self.parse_request(req).await?;

        Entity::write_filter(Entity::delete_by_id(Into::<i32>::into(req.id)), &auth)?
            .exec(db)
            .await
            .map_err(Into::<Error>::into)?;

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
            .ok_or(Error::NotInDB("user"))?;

        if !self.crypto.hash_eq(req.password.as_str(), &model.password) {
            return Err(Error::PremissionDeny("password").into());
        }

        for _ in 0..32 {
            let txn = db.begin().await.map_err(Into::<Error>::into)?;

            Entity::delete_by_id(user_id)
                .exec(&txn)
                .await
                .map_err(Into::<Error>::into)?;

            self.token.remove_by_user_id(user_id, &txn).await?;

            if txn.commit().await.map_err(Into::<Error>::into).is_ok() {
                return Ok(Response::new(()));
            }
        }
        Err(Status::aborted(
            "too many transaction retries, try again later",
        ))
    }
}
