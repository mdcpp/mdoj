mod pubsub;
mod route;
mod score;

use std::{
    ops::Deref,
    sync::{
        atomic::{AtomicI64, Ordering},
        Arc,
    },
};
use tokio_stream::StreamExt;

use crate::{
    grpc::TonicStream,
    init::{config, logger::PACKAGE_NAME},
    report_internal,
};
use futures::Future;
use leaky_bucket::RateLimiter;
use opentelemetry::{global, metrics::ObservableGauge};
use sea_orm::{ActiveModelTrait, ActiveValue, DatabaseConnection, EntityTrait, QueryOrder};
use thiserror::Error;
use tonic::Status;
use tracing::{instrument, Instrument, Span};
use uuid::Uuid;

use crate::grpc::{
    backend::{submit_status, JudgeResult as BackendResult, PlaygroundResult, SubmitStatus},
    judger::{JudgeResponse, *},
};

use self::{
    pubsub::{PubGuard, PubSub},
    route::*,
};
use super::code::Code;
use crate::entity::*;

const PALYGROUND_TIME: u64 = 500 * 1000;
const PALYGROUND_MEM: u64 = 256 * 1024 * 1024;

macro_rules! check_rate_limit {
    ($s:expr) => {{
        struct Waker;
        impl std::task::Wake for Waker {
            fn wake(self: Arc<Self>) {
                log::error!("waker wake");
            }
        }
        let waker = Arc::new(Waker).into();
        let mut cx = std::task::Context::from_waker(&waker);

        let ac = $s.limiter.clone().acquire_owned(1);
        tokio::pin!(ac);
        if ac.as_mut().poll(&mut cx).is_pending() {
            return Err(Error::RateLimit);
        }
    }};
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("judger temporarily unavailable")]
    JudgerResourceExhausted,
    #[error("`{0}`")]
    JudgerError(Status),
    #[error("`{0}`")]
    JudgerProto(&'static str),
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
            _ => Error::JudgerError(value),
        }
    }
}

impl From<Error> for Status {
    fn from(value: Error) -> Self {
        match value {
            Error::JudgerResourceExhausted => Status::resource_exhausted("no available judger"),
            Error::BadArgument(x) => Status::invalid_argument(format!("bad argument: {}", x)),
            Error::LangNotFound => Status::not_found("languaue not found"),
            Error::JudgerError(x) => report_internal!(info, "`{}`", x),
            Error::JudgerProto(x) => report_internal!(info, "`{}`", x),
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

impl From<i32> for SubmitStatus {
    fn from(value: i32) -> Self {
        SubmitStatus {
            task: Some(submit_status::Task::Case(value)),
        }
    }
}

impl From<Code> for SubmitStatus {
    fn from(value: Code) -> Self {
        SubmitStatus {
            task: Some(submit_status::Task::Result(
                Into::<Code>::into(value) as i32
            )),
        }
    }
}

struct MeterGuard<'a>(&'a JudgerController);

impl<'a> Drop for MeterGuard<'a> {
    fn drop(&mut self) {
        let (num, meter) = &self.0.running_meter;
        meter.observe(num.fetch_sub(1, Ordering::Acquire) - 1, &[]);
    }
}

pub struct PlaygroundPayload {
    pub input: Vec<u8>,
    pub code: Vec<u8>,
    pub lang: Uuid,
}

pub struct JudgerController {
    router: Arc<Router>,
    pubsub: Arc<PubSub<Result<SubmitStatus, Status>, i32>>,
    limiter: Arc<RateLimiter>,
    running_meter: (AtomicI64, ObservableGauge<i64>),
    db: Arc<DatabaseConnection>,
}

impl JudgerController {
    #[tracing::instrument(parent=span, name="judger_construct",level = "info",skip_all)]
    pub async fn new(
        config: Vec<config::Judger>,
        db: Arc<DatabaseConnection>,
        span: &Span,
    ) -> Result<Self, Error> {
        let router = Router::new(config, span)?;
        Ok(JudgerController {
            router,
            pubsub: Arc::new(PubSub::default()),
            limiter: Arc::new(
                RateLimiter::builder()
                    .max(25)
                    .initial(10)
                    .refill(2)
                    .interval(std::time::Duration::from_millis(100))
                    .build(),
            ),
            running_meter: (
                AtomicI64::new(0),
                global::meter(PACKAGE_NAME)
                    .i64_observable_gauge("running_judge")
                    .init(),
            ),
            db,
        })
    }
    fn record(&self) -> MeterGuard {
        let (num, meter) = &self.running_meter;
        meter.observe(num.fetch_add(1, Ordering::Acquire) + 1, &[]);

        MeterGuard(self)
    }
    #[instrument(skip(self, ps_guard, stream, model, scores))]
    async fn stream(
        self: Arc<Self>,
        ps_guard: PubGuard<Result<SubmitStatus, Status>, i32>,
        mut stream: tonic::Streaming<JudgeResponse>,
        mut model: submit::ActiveModel,
        scores: Vec<u32>,
        submit_id: i32,
    ) -> Result<submit::Model, Error> {
        let meter = self.record();
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
                .ok_or(Error::JudgerProto("Expected as many case as inputs"))??;
            total_memory += res.memory;
            total_time += res.time;
            total_score += score;
            match res.status() {
                JudgerCode::Ac => {}
                x => {
                    status = x.into();
                    break;
                }
            };
            pass_case += 1;
            if ps_guard
                .send(Ok(SubmitStatus {
                    task: Some(submit_status::Task::Case(pass_case)),
                }))
                .is_err()
            {
                tracing::trace!("no_rec");
            }
        }

        if ps_guard
            .send(Ok(SubmitStatus {
                task: Some(submit_status::Task::Result(
                    Into::<Code>::into(status) as i32
                )),
            }))
            .is_err()
        {
            tracing::trace!("no_rec");
        }

        model.committed = ActiveValue::Set(true);
        model.score = ActiveValue::Set(total_score);
        model.status = ActiveValue::Set(Some(status as u32));
        model.pass_case = ActiveValue::Set(pass_case);
        model.time = ActiveValue::Set(Some(total_time.try_into().unwrap_or(i64::MAX)));
        model.memory = ActiveValue::Set(Some(total_memory.try_into().unwrap_or(i64::MAX)));
        model.accept=ActiveValue::Set(status==Code::Accepted);

        drop(meter);

        model
            .update(self.db.deref())
            .in_current_span()
            .await
            .map_err(Into::<Error>::into)
    }
    pub async fn submit(self: &Arc<Self>, req: Submit) -> Result<i32, Error> {
        check_rate_limit!(self);
        let db = self.db.deref();

        let mut binding = problem::Entity::find_by_id(req.problem)
            .find_with_related(test::Entity)
            .order_by_asc(test::Column::Score)
            .all(db)
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
            ..Default::default()
        }
        .save(db)
        .await?;

        let submit_id = submit_model.id.as_ref().to_owned();
        let tx = self.pubsub.publish(submit_id);

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
        let db = self.db.clone();
        tokio::spawn(async move {
            match self_
                .stream(tx, res.into_inner(), submit_model, scores, submit_id)
                .await
            {
                Ok(submit) => {
                    score::ScoreUpload::new(
                        req.user,
                        problem,
                        submit.score,
                        submit.accept,
                    )
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
    pub async fn follow(&self, submit_id: i32) -> Option<TonicStream<SubmitStatus>> {
        self.pubsub.subscribe(&submit_id)
    }
    pub fn list_lang(&self) -> Vec<LangInfo> {
        self.router.langs.iter().map(|x| x.clone()).collect()
    }
    // endpoint should check uuid exist
    pub async fn playground(
        &self,
        payload: PlaygroundPayload,
    ) -> Result<TonicStream<PlaygroundResult>, Error> {
        check_rate_limit!(self);

        let mut conn = self.router.get(&payload.lang).await?;

        let res = conn
            .exec(ExecRequest {
                lang_uid: payload.lang.to_string(),
                code: payload.code,
                memory: PALYGROUND_MEM,
                time: PALYGROUND_TIME,
                input: payload.input,
            })
            .await?;

        conn.report_success();

        Ok(Box::pin(
            res.into_inner()
                .map(|x| x.map(Into::<PlaygroundResult>::into)),
        ))
    }
}
