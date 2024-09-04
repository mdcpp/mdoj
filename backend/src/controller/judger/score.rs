use std::cmp;

use super::{submit, user_contest};
use crate::{
    entity::{contest, problem, user},
    util::error::Error,
};
use chrono::Local;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel,
    ModelTrait, QueryFilter, QueryOrder, TransactionTrait,
};
use tracing::instrument;

const MAX_RETRY: usize = 32;

pub struct ScoreUpload {
    user_id: i32,
    problem: problem::Model,
    submit: submit::Model,
}

impl ScoreUpload {
    #[instrument(skip(problem))]
    pub fn new(user_id: i32, problem: problem::Model, submit: submit::Model) -> Self {
        Self {
            user_id,
            problem,
            submit,
        }
    }
    #[instrument(skip(self))]
    pub async fn upload(self, db: &DatabaseConnection) {
        let mut retries = MAX_RETRY;
        macro_rules! check {
            ($n:ident) => {
                paste::paste!{
                    while let Err(err) = (&self).[<upload_ $n>](db).await {
                        tracing::debug!(err = err.to_string(),entity=stringify!($n), "retry_upload");
                        match retries.checked_sub(1) {
                            None => {
                                tracing::warn!(err = err.to_string(), "tracscation_failed");
                                break;
                            }
                            Some(x) => {
                                retries = x;
                            }
                        }
                    }
                }
            };
        }
        check!(contest);
        check!(user);
        check!(problem);
    }
    async fn upload_problem(&self, db: &DatabaseConnection) -> Result<(), Error> {
        let mut model = self.problem.clone().into_active_model();
        let txn = db.begin().await?;

        let submit_count = model.submit_count.unwrap().saturating_add(1);
        let accept_count = match self.submit.accept {
            true => model.accept_count.unwrap().saturating_add(1),
            false => model.accept_count.unwrap(),
        };

        model.submit_count = ActiveValue::Set(submit_count);
        model.accept_count = ActiveValue::Set(accept_count);

        model.ac_rate = ActiveValue::Set(accept_count as f32 / submit_count as f32);

        txn.commit().await.map_err(Into::<Error>::into)
    }
    async fn upload_user(&self, db: &DatabaseConnection) -> Result<(), Error> {
        if !self.submit.accept {
            tracing::trace!(reason = "not acceptted", "score_user");
            return Ok(());
        }
        let txn = db.begin().await?;

        if self.user_id == self.problem.user_id {
            tracing::trace!(reason = "problem owner score bypass", "score_user");
            return Ok(());
        }
        if !self.problem.public {
            tracing::trace!(reason = "private problem score bypass", "score_user");
            return Ok(());
        }

        let user = self
            .problem
            .find_related(user::Entity)
            .one(&txn)
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB)?;

        let user_score = user.score;
        let mut user = user.into_active_model();

        user.score = ActiveValue::Set(user_score.saturating_add_unsigned(self.submit.score as u64));
        user.update(&txn).await.map_err(Into::<Error>::into)?;

        txn.commit().await.map_err(Into::<Error>::into)
    }
    async fn upload_contest(&self, db: &DatabaseConnection) -> Result<(), Error> {
        let txn = db.begin().await?;

        if self.user_id == self.problem.user_id {
            tracing::trace!(reason = "problem owner score bypass", "score_contest");
            return Ok(());
        }
        if self.problem.contest_id.is_none() {
            tracing::trace!(reason = "not under contest", "score_contest");
            return Ok(());
        }

        if self.submit.score == 0 {
            tracing::trace!(reason = "no score to add", "score_contest");
            return Ok(());
        }

        let contest_id = self.problem.contest_id.unwrap();

        let (contest, linker) = contest::Entity::find_by_id(contest_id)
            .find_also_related(user_contest::Entity)
            .one(&txn)
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB)?;

        let now = Local::now().naive_local();

        if let Some(end) = contest.end {
            if end < now {
                tracing::trace!(reason = "contest ended", "score_contest");
                return Ok(());
            }
        }

        if contest.host == self.user_id {
            tracing::trace!(reason = "owner score bypass", "score_contest");
            return Ok(());
        }

        let mut linker = linker
            .ok_or(Error::Unreachable("user_contest should exist"))?
            .into_active_model();

        let mut score = linker.score.unwrap();

        let submit = self
            .problem
            .find_related(submit::Entity)
            .filter(submit::Column::UserId.eq(self.user_id))
            .order_by_desc(submit::Column::Score)
            .one(&txn)
            .await?;

        let original_score = submit.map(|x| x.score).unwrap_or_default();

        if original_score >= self.submit.score {
            tracing::trace!(reason = "unchange score", "score_contest");
            return Ok(());
        }

        score = score.saturating_add(cmp::max(self.submit.score, original_score));
        score = score.saturating_sub(original_score);

        linker.score = ActiveValue::Set(score);
        linker.update(&txn).await.map_err(Into::<Error>::into)?;

        txn.commit().await.map_err(Into::<Error>::into)
    }
}
