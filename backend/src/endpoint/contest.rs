use super::tools::*;

use crate::entity::{
    contest::{Paginator, *},
    *,
};

use grpc::backend::contest_server::*;
use grpc::backend::*;

impl From<Model> for ContestFullInfo {
    fn from(value: Model) -> Self {
        ContestFullInfo {
            info: value.clone().into(),
            content: value.content,
            host: value.hoster.into(),
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
impl Contest for ArcServer {
    #[instrument(skip_all, level = "debug")]
    async fn list(
        &self,
        req: Request<ListContestRequest>,
    ) -> Result<Response<ListContestResponse>, Status> {
        let (auth, req) = self
            .parse_request_fn(req, |req| {
                ((req.size as u64) + req.offset.saturating_abs() as u64 / 5 + 2)
                    .try_into()
                    .unwrap_or(u32::MAX)
            })
            .await?;

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

        let list = paginator.fetch(req.size, req.offset).await?;
        let remain = paginator.remain().await?;

        let paginator = paginator.into_inner();

        Ok(Response::new(ListContestResponse {
            list: list.into_iter().map(Into::into).collect(),
            paginator: self.crypto.encode(paginator)?,
            remain,
        }))
    }
    #[instrument(skip_all, level = "debug")]
    async fn full_info(&self, req: Request<Id>) -> Result<Response<ContestFullInfo>, Status> {
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
    async fn create(&self, req: Request<CreateContestRequest>) -> Result<Response<Id>, Status> {
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

        let id: Id = model.id.clone().unwrap().into();
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
        let (user_id, perm) = auth.auth_or_guest()?;

        req.bound_check()?;

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
        let (user_id, perm) = auth.auth_or_guest()?;

        let model = Entity::read_filter(Entity::find_by_id(req.id), &auth)?
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

        tracing::debug!(user_id = user_id, contest_id = req.id);

        Ok(Response::new(()))
    }
}
