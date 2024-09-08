mod pubsub;
mod route;
mod score;

use std::{ops::Deref, sync::Arc};
use tokio_stream::StreamExt;

use crate::{report_internal, TonicStream};
use grpc::backend::StateCode as BackendCode;
use sea_orm::{ActiveModelTrait, ActiveValue, DatabaseConnection, EntityTrait, QueryOrder};
use thiserror::Error;
use tonic::Status;
use tracing::{instrument, Instrument};
use uuid::Uuid;

use self::{pubsub::PubSub, route::*};
use crate::config::CONFIG;
use crate::entity::*;
use crate::util::code::Code;
use grpc::{
    backend::{submit_status, SubmitStatus},
    judger::*,
};

#[derive(Debug, Error)]
pub enum Error {
    #[error("judger temporarily unavailable")]
    JudgerResourceExhausted,
    #[error("`{0}`")]
    Judger(Status),
    #[error("`{0}`")]
    JudgerProtoChanged(&'static str),
    #[error("payload.`{0}` is not a vaild argument")]
    BadArgument(&'static str),
    #[error("language not found")]
    LangNotFound,
    #[error("`{0}`")]
    Database(#[from] sea_orm::error::DbErr),
    #[error("`{0}`")]
    TransportLayer(#[from] tonic::transport::Error),
    #[error("Rate limit exceeded")]
    RateLimit,
    #[error("Dns resolve failed: `{0}`")]
    DnsResolve(#[from] hickory_resolver::error::ResolveError),
    #[error("uri parse failed: should be in format of `http://ip:port`")]
    UriParse,
}

impl From<Status> for Error {
    fn from(value: Status) -> Self {
        match value.code() {
            tonic::Code::ResourceExhausted => Error::JudgerResourceExhausted,
            _ => Error::Judger(value),
        }
    }
}

impl From<Error> for Status {
    fn from(value: Error) -> Self {
        match value {
            Error::JudgerResourceExhausted => Status::resource_exhausted("no available judger"),
            Error::BadArgument(x) => Status::invalid_argument(format!("bad argument: {}", x)),
            Error::LangNotFound => Status::not_found("languaue not found"),
            Error::Judger(x) => report_internal!(info, "`{}`", x),
            Error::JudgerProtoChanged(x) => report_internal!(info, "`{}`", x),
            Error::Database(x) => report_internal!(warn, "{}", x),
            Error::TransportLayer(x) => report_internal!(info, "{}", x),
            Error::RateLimit => Status::resource_exhausted("resource limit imposed by backend"),
            Error::DnsResolve(x) => report_internal!(warn, "{}", x),
            Error::UriParse => report_internal!(warn, "uri parse failed"),
        }
    }
}

#[derive(derive_builder::Builder)]
pub struct Submit {
    user: i32,
    problem: i32,
    time_limit: i64,
    memory_limit: i64,
    lang: Uuid,
    code: Vec<u8>,
}

impl From<Code> for SubmitStatus {
    fn from(value: Code) -> Self {
        SubmitStatus {
            task: Some(submit_status::Task::Result(
                Into::<BackendCode>::into(value) as i32,
            )),
        }
    }
}

#[allow(dead_code)]
pub struct PlaygroundPayload {
    pub input: Vec<u8>,
    pub code: Vec<u8>,
    pub lang: Uuid,
}

/// It manages state of upstream judger, provide ability to route request to potentially free upstream,
/// and provide enough publish-subscribe model
pub struct Judger {
    router: Arc<Router>,
    pubsub: Arc<PubSub<Result<SubmitStatus, Status>, i32>>,
    db: Arc<DatabaseConnection>,
}

impl Judger {
    #[tracing::instrument(name = "judger_construct", level = "info", skip_all)]
    pub async fn new(db: Arc<DatabaseConnection>) -> Result<Self, Error> {
        let judgers = CONFIG.judger.clone();
        let router = Router::new(judgers)?;
        Ok(Judger {
            router,
            pubsub: Arc::new(PubSub::default()),
            db,
        })
    }
    /// helper for streaming and process result(judge) from judger
    #[instrument(skip(self, stream, model, scores))]
    async fn stream(
        self: Arc<Self>,
        mut stream: tonic::Streaming<JudgeResponse>,
        mut model: submit::ActiveModel,
        scores: Vec<u32>,
    ) -> Result<submit::Model, Error> {
        let tx = self.pubsub.publish(*model.id.as_ref());

        let mut pass_case = 0;
        let mut status = Code::Accepted;
        let mut total_score = 0;
        let mut total_time = 0;
        let mut total_memory = 0;

        for score in scores.into_iter().rev() {
            let res = stream
                .next()
                .in_current_span()
                .await
                .ok_or(Error::JudgerProtoChanged("Expected as many case as inputs"))??;
            total_memory += res.memory;
            total_time += res.time;
            total_score += score;
            let res = res.status();
            if res != JudgerCode::Ac {
                status = res.into();
                break;
            }
            pass_case += 1;
            tx.send(Ok(SubmitStatus {
                task: Some(submit_status::Task::Case(pass_case)),
            }))
            .ok();
        }

        tx.send(Ok(SubmitStatus {
            task: Some(submit_status::Task::Result(
                Into::<BackendCode>::into(status) as i32,
            )),
        }))
        .ok();

        model.committed = ActiveValue::Set(true);
        model.score = ActiveValue::Set(total_score);
        model.status = ActiveValue::Set(Some(status as u32));
        model.pass_case = ActiveValue::Set(pass_case);
        model.time = ActiveValue::Set(Some(total_time.try_into().unwrap_or(i64::MAX)));
        model.memory = ActiveValue::Set(Some(total_memory.try_into().unwrap_or(i64::MAX)));
        model.accept = ActiveValue::Set(status == Code::Accepted);

        model
            .update(self.db.deref())
            .in_current_span()
            .await
            .map_err(Into::<Error>::into)
    }
    /// submit a problem
    pub async fn submit(self: &Arc<Self>, req: Submit) -> Result<i32, Error> {
        let db = self.db.clone();

        let mut binding = problem::Entity::find_by_id(req.problem)
            .find_with_related(testcase::Entity)
            .order_by_asc(testcase::Column::Score)
            .all(db.as_ref())
            .await?;
        let (problem, testcases) = binding.pop().ok_or(Error::BadArgument("problem id"))?;

        // create uncommited submit
        let submit_model = submit::ActiveModel {
            user_id: ActiveValue::Set(Some(req.user)),
            problem_id: ActiveValue::Set(req.user),
            committed: ActiveValue::Set(false),
            lang: ActiveValue::Set(req.lang.clone().to_string()),
            code: ActiveValue::Set(req.code.clone()),
            memory: ActiveValue::Set(Some(req.memory_limit)),
            public: ActiveValue::Set(problem.public),
            ..Default::default()
        }
        .save(db.as_ref())
        .await?;

        let submit_id = *submit_model.id.as_ref();

        let scores = testcases.iter().map(|x| x.score).collect::<Vec<_>>();

        let tests = testcases
            .into_iter()
            .map(|x| TestIo {
                input: x.input,
                output: x.output,
            })
            .collect::<Vec<_>>();

        let mut conn = self.router.get(&req.lang).await?;

        let res = conn
            .judge(JudgeRequest {
                lang_uid: req.lang.to_string(),
                code: req.code,
                memory: req.memory_limit as u64,
                time: req.time_limit as u64,
                rule: problem.match_rule,
                tests,
            })
            .await?;

        conn.report_success();

        let self_ = self.clone();
        tokio::spawn(async move {
            match self_.stream(res.into_inner(), submit_model, scores).await {
                Ok(submit) => {
                    score::ScoreUpload::new(req.user, problem, submit)
                        .upload(&db)
                        .await;
                }
                Err(err) => {
                    tracing::warn!(err = err.to_string(), "judge_fail");
                }
            }
        });

        Ok(submit_id)
    }
    /// abstraction for publish-subscribe
    pub fn follow(&self, submit_id: i32) -> Option<TonicStream<SubmitStatus>> {
        self.pubsub.subscribe(&submit_id)
    }
    pub fn list_lang(&self) -> Vec<LangInfo> {
        self.router.langs.iter().map(|x| x.clone()).collect()
    }
}
