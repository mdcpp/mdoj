use std::sync::Arc;

use sea_orm::{ActiveModelTrait, ActiveValue, EntityTrait, QueryOrder};
use thiserror::Error;
use tokio_stream::StreamExt;
use uuid::Uuid;

use crate::{
    grpc::{
        backend::{submit_status, JudgeResult as BackendResult, SubmitStatus},
        judger::{judge_response, JudgeRequest, JudgeResponse, JudgeResult, JudgerCode, TestIo},
    },
    init::{config::CONFIG, db::DB},
};

use super::util::{
    code::Code,
    pubsub::{PubGuard, PubSub},
    router::*,
};
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

impl From<Error> for super::Error {
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
    lang: Uuid,
    code: Vec<u8>,
}

impl Submit {
    async fn sent(self, ctrl: &SubmitController) -> Result<i32, Error> {
        ctrl.submit(self).await
    }
}

impl From<i32> for SubmitStatus {
    fn from(value: i32) -> Self {
        SubmitStatus {
            task: Some(submit_status::Task::Case(value)),
        }
    }
}

impl From<JudgeResult> for SubmitStatus {
    fn from(value: JudgeResult) -> Self {
        SubmitStatus {
            task: Some(submit_status::Task::Result(BackendResult {
                code: Into::<Code>::into(value.status()) as i32,
                accuracy: Some(value.accuracy),
                time: Some(value.time),
                memory: Some(value.memory),
            })),
        }
    }
}

#[derive(Clone)]
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
    async fn stream(
        ps_guard: PubGuard<Result<SubmitStatus, tonic::Status>, i32>,
        mut stream: tonic::Streaming<JudgeResponse>,
        mut model: submit::ActiveModel,
        mut scores: Vec<u32>,
    ) {
        let mut result = 0;
        let mut running_case = 0;
        let mut time = 0;
        let mut mem = 0;
        let mut status = JudgerCode::Ac;
        scores.reverse();
        while let Some(res) = stream.next().await {
            if res.is_err() {
                break;
            }
            let res = res.unwrap();
            if res.task.is_none() {
                log::warn!("mismatch proto(judger)");
                continue;
            }
            let task = res.task.unwrap();
            match task {
                judge_response::Task::Case(case) => {
                    if ps_guard.send(Ok(case.clone().into())).is_err() {
                        log::trace!("client disconnected");
                    }
                    if case != (running_case + 1) {
                        log::warn!("mismatch proto(judger)");
                    }
                    running_case += 1;
                }
                judge_response::Task::Result(case) => {
                    if let Some(score) = scores.pop() {
                        if ps_guard.send(Ok(case.clone().into())).is_err() {
                            log::trace!("client disconnected");
                        }
                        if case.status() == JudgerCode::Ac {
                            result += score;
                        } else {
                            status = case.status();
                            mem += case.memory;
                            time += case.time;
                            break;
                        }
                    } else {
                        log::warn!("mismatch proto(judger), too many cases");
                    }
                }
            }
        }
        model.committed = ActiveValue::Set(true);
        model.score = ActiveValue::Set(result);
        model.status = ActiveValue::Set(Into::<Code>::into(status) as u32);
        model.pass_case = ActiveValue::Set(running_case);
        model.time = ActiveValue::Set(Some(time));
        model.memory = ActiveValue::Set(Some(mem));

        if let Err(err) = model.update(DB.get().unwrap()).await {
            log::warn!("failed to commit the judge result: {}", err);
        }
    }
    pub async fn submit(&self, submit: Submit) -> Result<i32, Error> {
        let db = DB.get().unwrap();

        let mut conn = self.router.get(&submit.lang).await?;

        let mut binding = problem::Entity::find_by_id(submit.problem)
            .find_with_related(test::Entity)
            .order_by_asc(test::Column::Score)
            .all(db)
            .await?;
        let (problem, testcases) = binding.pop().ok_or(Error::BadArgument("problem id"))?;

        // create uncommited submit
        let submit_model = submit::ActiveModel {
            user_id: ActiveValue::Set(Some(submit.user)),
            problem_id: ActiveValue::Set(submit.user),
            committed: ActiveValue::Set(false),
            lang: ActiveValue::Set(submit.lang.clone().to_string()),
            code: ActiveValue::Set(submit.code.clone()),
            memory: ActiveValue::Set(Some(submit.memory_limit)),
            ..Default::default()
        }
        .save(db)
        .await?;

        let submit_id = submit_model.id.as_ref().to_owned();
        let tx = self.pubsub.publish(submit_id);

        let scores = testcases.iter().rev().map(|x| x.score).collect::<Vec<_>>();

        let tests = testcases
            .into_iter()
            .map(|x| TestIo {
                input: x.input,
                output: x.output,
            })
            .collect::<Vec<_>>();

        let res = conn
            .judge(JudgeRequest {
                lang_uid: submit.lang.to_string(),
                code: submit.code,
                memory: submit.memory_limit,
                time: submit.time_limit,
                rule: problem.match_rule,
                tests,
            })
            .await?;

        tokio::spawn(Self::stream(tx, res.into_inner(), submit_model, scores));

        Ok(submit_id)
    }
    pub async fn follow(&self, submit_id: i32) -> Option<TonicStream<SubmitStatus>> {
        self.pubsub.subscribe(&submit_id)
    }
    // pub async fn rejudge(&self, submit_id: i32) -> Result<(), Error> {
    //     let db = DB.get().unwrap();
    //     let model =
    //         submit::Entity::find_by_id(submit_id)
    //             .one(db)
    //             .await?
    //             .ok_or(Error::GrpcReport(tonic::Status::failed_precondition(
    //                 "rejudge error: cannot find submit to rejudge",
    //             )))?;

    //     let lang = Uuid::parse_str(model.lang.as_str()).map_err(Into::<Error>::into)?;
    //     let code = model.code;
    //     let memory = model.memory;
    //     let time = model.time;

    //     // tokio::spawn(Self::stream(tx, res.into_inner(), submit_model, scores));

    //     Ok(())
    // }
}

impl From<Error> for tonic::Status {
    fn from(value: Error) -> Self {
        match value {
            Error::JudgerUnavailable => todo!(),
            Error::HealthCheck => todo!(),
            Error::GrpcReport(_) => todo!(),
            Error::BadArgument(_) => todo!(),
            Error::Database(_) => todo!(),
            Error::Tonic(_) => todo!(),
            Error::Internal(_) => todo!(),
        }
    }
}
