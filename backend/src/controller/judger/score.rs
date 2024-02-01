use super::{user_contest, DebugName};
use crate::{
    entity::{contest, problem, user},
    util::error::Error,
};
use sea_orm::{
    ActiveModelTrait, ActiveValue, DatabaseConnection, EntityTrait, IntoActiveModel, ModelTrait,
    TransactionTrait,
};
use tracing::instrument;

const MAX_RETRY: usize = 32;

pub struct ScoreUpload {
    user_id: i32,
    problem: problem::Model,
    score: u32,
}

impl ScoreUpload {
    #[instrument(skip(problem))]
    pub fn new(user_id: i32, problem: problem::Model, score: u32) -> Self {
        Self {
            user_id,
            problem,
            score,
        }
    }
    #[instrument(skip(self))]
    pub async fn upload(self, db: &DatabaseConnection) {
        let self_ = self;
        let mut retries = MAX_RETRY;
        while let Err(err) = self_.upload_contest(db).await {
            tracing::debug!(err = err.to_string(), "retry_upload");
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
        while let Err(err) = self_.upload_user(db).await {
            tracing::debug!(err = err.to_string(), "retry_upload");
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
    async fn upload_user(&self, db: &DatabaseConnection) -> Result<(), Error> {
        let txn = db.begin().await?;

        if self.user_id == self.problem.user_id {
            return Ok(());
        }
        if !self.problem.public {
            return Ok(());
        }

        let user = self
            .problem
            .find_related(user::Entity)
            .one(&txn)
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB(user::Entity::DEBUG_NAME))?;

        let user_score = user.score;
        let mut user = user.into_active_model();

        user.score = ActiveValue::Set(self.score + user_score);
        user.update(&txn).await.map_err(Into::<Error>::into)?;

        txn.commit().await.map_err(Into::<Error>::into)
    }
    async fn upload_contest(&self, db: &DatabaseConnection) -> Result<(), Error> {
        let txn = db.begin().await?;

        if self.user_id == self.problem.user_id {
            return Ok(());
        }
        if self.problem.contest_id.is_none() {
            return Ok(());
        }

        let contest_id = self.problem.contest_id.unwrap();

        let (contest, linker) = contest::Entity::find_by_id(contest_id)
            .find_also_related(user_contest::Entity)
            .one(&txn)
            .await
            .map_err(Into::<Error>::into)?
            .ok_or(Error::NotInDB(user::Entity::DEBUG_NAME))?;

        if contest.hoster == self.user_id {
            return Ok(());
        }

        let mut linker = linker
            .ok_or(Error::Unreachable("user_contest should exist"))?
            .into_active_model();

        linker.score = ActiveValue::Set(self.score);
        linker.update(&txn).await.map_err(Into::<Error>::into)?;

        txn.commit().await.map_err(Into::<Error>::into)
    }
}
