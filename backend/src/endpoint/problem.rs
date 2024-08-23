use super::*;
use grpc::backend::problem_server::*;
use sea_orm::sea_query::Expr;

use crate::entity::{contest, problem::Paginator, problem::*, testcase};

impl<'a> From<WithAuth<'a, Model>> for ProblemFullInfo {
    fn from(value: WithAuth<'a, Model>) -> Self {
        let model = value.1;
        let writable = Entity::writable(&model, value.0);
        ProblemFullInfo {
            content: model.content.clone(),
            tags: model.tags.clone(),
            difficulty: model.difficulty,
            public: model.public,
            time: model.time as u64,
            memory: model.memory as u64,
            info: ProblemInfo {
                id: model.id,
                title: model.title,
                submit_count: model.submit_count,
                ac_rate: model.ac_rate,
                difficulty: model.difficulty,
            },
            author: model.user_id,
            writable,
        }
    }
}

impl WithAuthTrait for Model {}

impl From<PartialModel> for ProblemInfo {
    fn from(value: PartialModel) -> Self {
        ProblemInfo {
            id: value.id,
            title: value.title,
            submit_count: value.submit_count,
            ac_rate: value.ac_rate,
            difficulty: value.difficulty,
        }
    }
}

#[async_trait]
impl Problem for ArcServer {
    #[instrument(
        skip_all,
        level = "info",
        name = "oj.backend.Problem/list",
        err(level = "debug", Display)
    )]
    async fn list(
        &self,
        req: Request<ListProblemRequest>,
    ) -> Result<Response<ListProblemResponse>, Status> {
        let (auth, req) = self.rate_limit(req).in_current_span().await?;

        req.bound_check()?;

        let paginator = match req.request.ok_or(Error::NotInPayload("request"))? {
            list_problem_request::Request::Create(create) => {
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
            list_problem_request::Request::Paginator(x) => self.crypto.decode(x)?,
        };
        let mut paginator = paginator.with_auth(&auth).with_db(&self.db);

        let list = paginator
            .fetch(req.size, req.offset)
            .in_current_span()
            .await?;
        let remain = paginator.remain().in_current_span().await?;

        let paginator = paginator.into_inner();

        Ok(Response::new(ListProblemResponse {
            list: list.into_iter().map(Into::into).collect(),
            paginator: self.crypto.encode(paginator)?,
            remain,
        }))
    }
    #[instrument(
        skip_all,
        level = "info",
        name = "oj.backend.Problem/full_info",
        err(level = "debug", Display)
    )]
    async fn full_info(&self, req: Request<Id>) -> Result<Response<ProblemFullInfo>, Status> {
        let (auth, req) = self.rate_limit(req).in_current_span().await?;

        let query = Entity::find_by_id::<i32>(req.into())
            .with_auth(&auth)
            .read()?;
        let model = query
            .one(self.db.deref())
            .instrument(info_span!("fetch"))
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB)?;

        Ok(Response::new(model.with_auth(&auth).into()))
    }
    #[instrument(
        skip_all,
        level = "info",
        name = "oj.backend.Problem/create",
        err(level = "debug", Display)
    )]
    async fn create(&self, req: Request<CreateProblemRequest>) -> Result<Response<Id>, Status> {
        let (auth, req) = self.rate_limit(req).in_current_span().await?;
        req.bound_check()?;

        let (user_id, perm) = auth.assume_login()?;
        perm.super_user()?;

        req.get_or_insert(|req| async move {
            let mut model: ActiveModel = Default::default();
            model.user_id = ActiveValue::Set(user_id);

            fill_active_model!(
                model, req.info, title, difficulty, time, memory, tags, content, match_rule, order
            );

            let model = model
                .save(self.db.deref())
                .instrument(info_span!("save").or_current())
                .await
                .map_err(Into::<Error>::into)?;

            let id = *model.id.as_ref();

            info!(count.problem = 1, id = id);

            Ok(id.into())
        })
        .await
        .with_grpc()
        .into()
    }
    #[instrument(
        skip_all,
        level = "info",
        name = "oj.backend.Problem/update",
        err(level = "debug", Display)
    )]
    async fn update(&self, req: Request<UpdateProblemRequest>) -> Result<Response<()>, Status> {
        let (auth, req) = self.rate_limit(req).in_current_span().await?;

        req.bound_check()?;

        req.get_or_insert(|req| async move {
            let mut model = Entity::find_by_id(req.id)
                .with_auth(&auth)
                .write()?
                .one(self.db.deref())
                .await
                .map_err(Into::<Error>::into)?
                .ok_or(Error::NotInDB)?
                .into_active_model();

            fill_exist_active_model!(
                model, req.info, title, difficulty, time, memory, tags, content, match_rule, order
            );

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
        name = "oj.backend.Problem/remove",
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
                .await?;

            if result.rows_affected == 0 {
                return Err(Error::NotInDB);
            }
            info!(count.problem = -1, id = req.id);
            Ok(())
        })
        .await
        .with_grpc()
        .into()
    }
    #[instrument(
        skip_all,
        level = "info",
        name = "oj.backend.Problem/add_to_contest",
        err(level = "debug", Display)
    )]
    async fn add_to_contest(
        &self,
        req: Request<AddProblemToContestRequest>,
    ) -> Result<Response<()>, Status> {
        let (auth, req) = self.rate_limit(req).in_current_span().await?;

        req.get_or_insert(|req| async move {
            let (contest, model) = tokio::try_join!(
                contest::Entity::write_by_id(req.contest_id, &auth)?
                    .into_partial_model()
                    .one(self.db.deref())
                    .instrument(debug_span!("find_parent").or_current()),
                Entity::write_by_id(req.problem_id, &auth)?
                    .one(self.db.deref())
                    .instrument(debug_span!("find_child").or_current())
            )
            .map_err(Into::<Error>::into)?;

            let contest: contest::IdModel = contest.ok_or(Error::NotInDB)?;

            let mut model = model.ok_or(Error::NotInDB)?.into_active_model();
            if let ActiveValue::Set(Some(v)) = model.contest_id {
                return Err(Error::AlreadyExist("problem already linked"));
            }

            let order = contest
                .with_db(self.db.deref())
                .insert_last()
                .await
                .map_err(Into::<Error>::into)?;
            model.order = ActiveValue::Set(order);

            model.contest_id = ActiveValue::Set(Some(req.problem_id));
            model
                .save(self.db.deref())
                .instrument(info_span!("update_child").or_current())
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
        name = "oj.backend.Problem/remove_from_contest",
        err(level = "debug", Display)
    )]
    async fn remove_from_contest(
        &self,
        req: Request<AddProblemToContestRequest>,
    ) -> Result<Response<()>, Status> {
        let (auth, req) = self.rate_limit(req).in_current_span().await?;
        auth.perm().super_user()?;

        req.get_or_insert(|req| async move {
            let (contest, model) = tokio::try_join!(
                contest::Entity::write_by_id(req.contest_id, &auth)?
                    .one(self.db.deref())
                    .instrument(debug_span!("find_parent").or_current()),
                Entity::write_by_id(req.problem_id, &auth)?
                    .one(self.db.deref())
                    .instrument(debug_span!("find_child").or_current())
            )
            .map_err(Into::<Error>::into)?;

            contest.ok_or(Error::NotInDB)?;

            let mut model = model.ok_or(Error::NotInDB)?.into_active_model();
            if let Some(x) = model.contest_id.into_value() {
                debug!(old_id = x.to_string());
            }
            model.contest_id = ActiveValue::Set(None);
            model
                .save(self.db.deref())
                .instrument(info_span!("update_child").or_current())
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
        name = "oj.backend.Problem/insert",
        err(level = "debug", Display)
    )]
    async fn insert(&self, req: Request<InsertProblemRequest>) -> Result<Response<()>, Status> {
        let (auth, req) = self.rate_limit(req).in_current_span().await?;
        auth.perm().super_user()?;

        req.get_or_insert(|req| async move {
            let contest: contest::IdModel = contest::Entity::find_by_id(req.contest_id)
                .with_auth(&auth)
                .write()?
                .into_partial_model()
                .one(self.db.deref())
                .instrument(info_span!("fetch").or_current())
                .await
                .map_err(Into::<Error>::into)?
                .ok_or(Error::NotInDB)?;

            let order = match req.pivot_id {
                None => contest.with_db(self.db.deref()).insert_front().await,
                Some(id) => contest.with_db(self.db.deref()).insert_after(id).await,
            }
            .map_err(Into::<Error>::into)?;

            Entity::write_filter(
                Entity::update(ActiveModel {
                    id: ActiveValue::Set(req.problem_id),
                    order: ActiveValue::Set(order),
                    ..Default::default()
                }),
                &auth,
            )?
            .exec(self.db.deref())
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
        name = "oj.backend.Problem/publish",
        err(level = "debug", Display)
    )]
    async fn publish(&self, req: Request<PublishRequest>) -> Result<Response<()>, Status> {
        let (auth, req) = self.rate_limit(req).in_current_span().await?;

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
        name = "oj.backend.Problem/unpublish",
        err(level = "debug", Display)
    )]
    async fn unpublish(&self, req: Request<PublishRequest>) -> Result<Response<()>, Status> {
        let (auth, req) = self.rate_limit(req).in_current_span().await?;

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
    #[instrument(
        skip_all,
        level = "info",
        name = "oj.backend.Problem/full_info_by_contest",
        err(level = "debug", Display)
    )]
    async fn full_info_by_contest(
        &self,
        req: Request<ListProblemByContestRequest>,
    ) -> Result<Response<ProblemFullInfo>, Status> {
        let (auth, req) = self.rate_limit(req).in_current_span().await?;

        let parent: contest::IdModel =
            contest::Entity::related_read_by_id(&auth, req.contest_id, &self.db)
                .in_current_span()
                .await?;

        let model = parent
            .upgrade()
            .find_related(Entity)
            .filter(Column::Id.eq(req.problem_id))
            .one(self.db.deref())
            .instrument(info_span!("fetch").or_current())
            .in_current_span()
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB)?;

        Ok(Response::new(model.with_auth(&auth).into()))
    }
}
