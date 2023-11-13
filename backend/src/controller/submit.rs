use sea_orm::{ActiveModelTrait, ActiveValue};
use thiserror::Error;

use crate::endpoint::tools::DB;

use super::util::router::*;
use entity::submit::*;

pub struct SubmitController {
    router: Router,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("judger temporarily unavailable")]
    JudgerUnavailable,
    #[error("judger health check failed")]
    HealthCheck,
    #[error("judger reach limit")]
    ReachLimit,
    #[error("payload.`{0}` is not a vaild argument")]
    BadArgument(&'static str),
    #[error("`{0}`")]
    Database(#[from] sea_orm::error::DbErr),
    #[error("`{0}`")]
    Tonic(#[from] tonic::transport::Error),
    #[error("`{0}`")]
    Internal(&'static str),
    // #[error("judger tls error")]
    // TlsError,
}

impl Into<super::Error> for Error {
    fn into(self) -> super::Error {
        match self {
            Error::JudgerUnavailable => super::Error::Internal("judger temporarily unavailable"),
            Error::HealthCheck => super::Error::Internal("judger health check failed"),
            Error::ReachLimit => tonic::Status::unavailable("judger reach limit").into(),
            Error::BadArgument(x) => tonic::Status::invalid_argument(format!(
                "Client sent invaild argument: payload.{}",
                x
            ))
            .into(),
            Error::Database(x) => super::Error::Database(x),
            Error::Tonic(x) => super::Error::Tonic(x),
            Error::Internal(x) => super::Error::Internal(x),
            // Error::TlsError => todo!(),
        }
    }
}

#[derive(derive_builder::Builder)]
pub struct Submit {
    user: i32,
    problem: i32,
    time_limit: i64,
    memory_limit: i64,
    lang: String,
    code: Vec<u8>,
}

impl Submit {
    async fn sent(self, ctrl: &SubmitController) -> Result<i32, Error> {
        ctrl.submit(self).await
    }
}

impl SubmitController {
    async fn submit(&self, submit: Submit) -> Result<i32, Error> {
        let db = DB.get().unwrap();

        let model = ActiveModel {
            user_id: ActiveValue::Set(submit.user),
            problem_id: ActiveValue::Set(submit.user),
            committed: ActiveValue::Set(false),
            lang: ActiveValue::Set(submit.lang),
            code: ActiveValue::Set(submit.code),
            memory: ActiveValue::Set(Some(submit.memory_limit)),
            ..Default::default()
        };

        let mut model=model.save(db).await?;

        todo!();

        model.id.take().ok_or(Error::Internal("submit id should be autoincrement"))
    }
}
