use std::{borrow::BorrowMut, sync::Arc};

use sea_orm::{ActiveModelTrait, ActiveValue, EntityTrait, IntoActiveModel, QueryOrder};
use thiserror::Error;
use tokio_stream::StreamExt;

use crate::{
    grpc::{
        backend::SubmitStatus,
        judger::{JudgeRequest, TestIo},
    },
    init::{config::CONFIG, db::DB},
};

use super::util::{pubsub::PubSub, router::*};
use entity::*;

type TonicStream<T> =
    std::pin::Pin<Box<dyn tokio_stream::Stream<Item = Result<T, tonic::Status>> + Send>>;

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

impl From<Error> for super::Error{
    fn from(value: Error) -> Self {
        match value {
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

pub struct SubmitController {
    router: Arc<Router>,
    pubsub: Arc<PubSub<Result<SubmitStatus, tonic::Status>, i32>>,
}

impl SubmitController {
    pub async fn new() -> Result<Self, Error> {
        let config = CONFIG.get().unwrap();
        Ok(SubmitController {
            router: Router::new(&config.judger).await?,
            pubsub: Arc::new(PubSub::new()),
        })
    }
    async fn submit(&self, submit: Submit) -> Result<i32, Error> {
        let db = DB.get().unwrap();

        let mut conn = self.router.get(&submit.lang).await?;

        let mut binding = problem::Entity::find_by_id(submit.problem)
            .find_with_related(test::Entity)
            .order_by_asc(test::Column::Score)
            .all(db)
            .await?;
        let (mut problem, testcases) = binding.pop().ok_or(Error::BadArgument("problem id"))?;

        // create uncommited submit
        let submit_model = submit::ActiveModel {
            user_id: ActiveValue::Set(Some(submit.user)),
            problem_id: ActiveValue::Set(submit.user),
            committed: ActiveValue::Set(false),
            lang: ActiveValue::Set(submit.lang.clone()),
            code: ActiveValue::Set(submit.code.clone()),
            memory: ActiveValue::Set(Some(submit.memory_limit)),
            ..Default::default()
        }
        .save(db)
        .await?;

        let submit_id = submit_model.id.as_ref().to_owned();
        let mut pubguard = self.pubsub.publish(submit_id);

        let mut scores = testcases.iter().rev().map(|x| x.score).collect::<Vec<_>>();

        let tests = testcases
            .into_iter()
            .map(|x| TestIo {
                input: x.input,
                output: x.output,
            })
            .collect::<Vec<_>>();

        let res = conn
            .judge(JudgeRequest {
                lang_uid: submit.lang,
                code: submit.code,
                memory: submit.memory_limit,
                time: submit.time_limit,
                rule: problem.match_rule,
                tests,
            })
            .await?;

        tokio::spawn(async move {
            let mut state = crate::controller::util::state::State::default();
            let mut res = res.into_inner();

            while let Some(res) = res.next().await {
                match res {
                    Ok(res) => {
                        state.parse_state(pubguard.borrow_mut(), res);
                    }
                    Err(err) => {
                        log::warn!("{}", err);
                        pubguard.send(Err(err)).ok();
                        break;
                    }
                }
            }
            let pass = state.pass == scores.len();
            let mut score = 0;
            while let Some(x) = scores.pop() {
                if state.pass == 0 {
                    break;
                }
                state.pass -= 1;
                score += x;
            }

            let mut submit_model = submit_model.into_active_model();
            submit_model.committed = ActiveValue::Set(true);
            submit_model.time = ActiveValue::Set(Some(state.time));
            submit_model.memory = ActiveValue::Set(Some(state.mem));
            submit_model.score = ActiveValue::Set(score);

            problem.submit_count += 1;
            if pass {
                problem.accept_count += 1;
            }
            problem.ac_rate = problem.accept_count as f32 / problem.submit_count as f32;

            if let Err(err) = problem.into_active_model().save(db).await {
                log::warn!("failed to update problem statistics: {}", err);
            }
            if let Err(err) = submit_model.save(db).await {
                log::warn!("failed to commit the judge result: {}", err);
            }
        });
        Ok(submit_id)
    }
    async fn follow(&self, submit_id: i32) -> Option<TonicStream<SubmitStatus>> {
        self.pubsub.subscribe(&submit_id)
    }
}
