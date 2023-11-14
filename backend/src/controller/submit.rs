use std::{ops::Deref, sync::Arc, borrow::BorrowMut};

use sea_orm::{ActiveModelTrait, ActiveValue, EntityTrait, Related, IntoActiveModel};
use thiserror::Error;
use tokio_stream::StreamExt;

use crate::{
    controller::util::state::parse_state,
    grpc::{
        backend::SubmitStatus,
        judger::{JudgeRequest, TestIo},
    },
    init::db::DB,
};

use super::util::{pubsub::PubSub, router::*};
use entity::*;

pub struct SubmitController {
    router: Router,
    pubsub: Arc<PubSub<SubmitStatus, i32>>,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("judger temporarily unavailable")]
    JudgerUnavailable,
    #[error("judger health check failed")]
    HealthCheck,
    #[error("`{0}`")]
    GrpcReport(#[from] tonic::Status),
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
            Error::JudgerUnavailable => {
                super::Error::Internal("no judger available(for such lang)")
            }
            Error::HealthCheck => super::Error::Internal("judger health check failed"),
            Error::BadArgument(x) => tonic::Status::invalid_argument(format!(
                "Client sent invaild argument: payload.{}",
                x
            ))
            .into(),
            Error::Database(x) => super::Error::Database(x),
            Error::Tonic(x) => super::Error::Tonic(x),
            Error::Internal(x) => super::Error::Internal(x),
            Error::GrpcReport(x) => super::Error::GrpcReport(x),
        }
    }
}

#[derive(derive_builder::Builder)]
pub struct Submit {
    user: i32,
    problem: i32,
    time_limit: u64,
    memory_limit: u64,
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

        let mut conn = self.router.get(&submit.lang).await?;

        let (problem, testcases) = problem::Entity::find_by_id(submit.problem)
            .find_also_related(testcase::Entity)
            .one(db)
            .await?
            .ok_or(Error::BadArgument("problem id"))?;

        // create uncommited submit
        let model = submit::ActiveModel {
            user_id: ActiveValue::Set(submit.user),
            problem_id: ActiveValue::Set(submit.user),
            committed: ActiveValue::Set(false),
            lang: ActiveValue::Set(submit.lang.clone()),
            code: ActiveValue::Set(submit.code.clone()),
            memory: ActiveValue::Set(Some(submit.memory_limit)),
            ..Default::default()
        }
        .save(db)
        .await?;

        let submit_id = model.id.as_ref().clone();
        let mut pubguard = self.pubsub.publish(submit_id);

        let tests = testcases
            .into_iter()
            .map(|x| TestIo {
                input: x.stdin,
                output: x.stdout,
            })
            .collect::<Vec<_>>();

        let res = conn
            .judge(JudgeRequest {
                lang_uid: submit.lang,
                code: submit.code,
                memory: submit.memory_limit,
                time: submit.time_limit as u64,
                rule: problem.match_rule,
                tests,
            })
            .await?;

        tokio::spawn(async move {
            let mut res = res.into_inner();

            while let Some(res) = res.next().await {
                if let Ok(res) = res.map_err(|x| log::warn!("{}", x)) {
                    todo!("count time and peak memory");
                    parse_state(pubguard.borrow_mut(), res);
                }
            }
            todo!("commit the judge result");
            let mut model=model.into_active_model();
            model.committed=ActiveValue::Set(true);
            // model.time=ActiveValue::Set(res.time);
        });
        Ok(submit_id)
    }
    async fn follow(&self, submit_id: i32) {
        todo!()
    }
}
