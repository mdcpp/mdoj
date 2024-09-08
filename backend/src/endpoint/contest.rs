use super::*;
use crate::entity::{
    contest::{Paginator, *},
    problem, *,
};
use chrono::Local;
use sea_orm::sea_query::Expr;

use grpc::backend::contest_server::*;
use grpc::backend::update_contest_request::info::{End, Password};

impl<'a> From<WithAuth<'a, Model>> for ContestFullInfo {
    fn from(value: WithAuth<'a, Model>) -> Self {
        let model = value.1;
        let writable = Entity::writable(&model, value.0);
        ContestFullInfo {
            info: model.clone().into(),
            content: model.content,
            host: model.host,
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
            begin: value.begin.map(into_prost),
            end: value.end.map(into_prost),
            need_password: value.password.is_some(),
        }
    }
}

impl From<PartialModel> for ContestInfo {
    fn from(value: PartialModel) -> Self {
        ContestInfo {
            id: value.id,
            title: value.title,
            begin: value.begin.map(into_prost),
            end: value.end.map(into_prost),
            need_password: value.password.is_some(),
        }
    }
}

#[async_trait]
impl Contest for ArcServer {
    #[instrument(
        skip_all,
        level = "info",
        name = "oj.backend.Contest/list",
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
        name = "oj.backend.Contest/full_info",
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
        name = "oj.backend.Contest/create",
        err(level = "debug", Display)
    )]
    async fn create(&self, req: Request<CreateContestRequest>) -> Result<Response<Id>, Status> {
        let (auth, req) = self.rate_limit(req).in_current_span().await?;
        req.bound_check()?;

        let (user_id, perm) = auth.assume_login()?;
        perm.super_user()?;

        req.get_or_insert(|req| async {
            let mut model: ActiveModel = Default::default();
            model.host = ActiveValue::Set(user_id);

            fill_active_model!(model, req.info, title, content, tags);

            if let Some(password) = req.info.password {
                let password: Vec<u8> = self.crypto.hash(password.as_str());
                model.password = ActiveValue::Set(Some(password));
            }

            model.end = req.info.end.map(into_chrono).into_active_value();

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

            info!(count.contest.count = 1, id = id);

            Ok(id.into())
        })
        .await
        .with_grpc()
        .into()
    }
    #[instrument(
        skip_all,
        level = "info",
        name = "oj.backend.Contest/update",
        err(level = "debug", Display)
    )]
    async fn update(&self, req: Request<UpdateContestRequest>) -> Result<Response<()>, Status> {
        let (auth, req) = self.rate_limit(req).in_current_span().await?;
        req.bound_check()?;

        let (_, _perm) = auth.assume_login()?;

        req.get_or_insert(|req| async move {
            trace!(id = req.id);

            let mut model = Entity::find_by_id(req.id)
                .with_auth(&auth)
                .write()?
                .one(self.db.deref())
                .instrument(info_span!("fetch").or_current())
                .await
                .map_err(Into::<Error>::into)?
                .ok_or(Error::NotInDB)?;
            match req.info.password {
                Some(Password::PasswordUnset(_)) => {
                    model.password = None;
                }
                Some(Password::PasswordSet(password)) => {
                    let hash = self.crypto.hash(&password);
                    model.password = Some(hash);
                }
                _ => {}
            }

            let mut model = model.into_active_model();

            fill_exist_active_model!(model, req.info, title, content, tags);
            match req.info.end {
                Some(End::EndSet(x)) => {
                    model.end = ActiveValue::Set(Some(into_chrono(x)));
                }
                Some(End::EndUnset(_)) => {
                    model.end = ActiveValue::NotSet;
                }
                _ => {}
            }

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
        name = "oj.backend.Contest/remove",
        err(level = "debug", Display)
    )]
    async fn remove(&self, req: Request<RemoveRequest>) -> Result<Response<()>, Status> {
        let (auth, req) = self.rate_limit(req).in_current_span().await?;

        req.get_or_insert(|req| async move {
            let txn = self.db.begin().await?;

            let result = Entity::delete_by_id(req.id)
                .with_auth(&auth)
                .write()?
                .exec(self.db.deref())
                .instrument(info_span!("remove").or_current())
                .await
                .map_err(Into::<Error>::into)?;

            problem::Entity::update_many()
                .col_expr(problem::Column::ContestId, Expr::value(Value::Int(None)))
                .filter(crate::entity::testcase::Column::ProblemId.eq(req.id))
                .exec(&txn)
                .instrument(info_span!("remove_child"))
                .await?;

            txn.commit().await.map_err(|_| Error::Retry)?;

            if result.rows_affected == 0 {
                return Err(Error::NotInDB);
            }
            info!(counter.contest = -1, id = req.id);
            Ok(())
        })
        .await
        .with_grpc()
        .into()
    }
    #[instrument(
        skip_all,
        level = "info",
        name = "oj.backend.Contest/publish",
        err(level = "debug", Display)
    )]
    async fn publish(&self, req: Request<PublishContestRequest>) -> Result<Response<()>, Status> {
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
            if let Some(begin) = req.begin.map(into_chrono) {
                model.begin = ActiveValue::Set(Some(begin));
            }

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
        name = "oj.backend.Contest/unpublish",
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
    #[instrument(
        skip_all,
        level = "info",
        name = "oj.backend.Contest/join",
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

            if let Some(tar) = model.password {
                let password = req
                    .password
                    .as_ref()
                    .ok_or(Error::NotInPayload("password"))?;
                if !self.crypto.hash_eq(password, &tar) {
                    return Err(Error::PermissionDeny("mismatched password"));
                }
            }

            if let Some(begin) = model.begin {
                if begin > Local::now().naive_local() {
                    return Err(Error::PermissionDeny("contest not begin"));
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

            debug!(user_id = user_id, contest_id = req.id);
            Ok(())
        })
        .await
        .with_grpc()
        .into()
    }
}
