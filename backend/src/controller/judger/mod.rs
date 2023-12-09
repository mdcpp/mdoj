mod pubsub;
mod route;
use std::sync::{
    atomic::{AtomicI64, Ordering},
    Arc,
};

use crate::{
    grpc::TonicStream,
    init::{config, logger::PACKAGE_NAME},
    report_internal,
};
use futures::Future;
use leaky_bucket::RateLimiter;
use opentelemetry::{global, metrics::ObservableGauge};
use sea_orm::{ActiveModelTrait, ActiveValue, EntityTrait, QueryOrder};
use thiserror::Error;
use tokio_stream::StreamExt;
use tonic::Status;
use tracing::{instrument, Span};
use uuid::Uuid;

use crate::{
    grpc::{
        backend::{submit_status, JudgeResult as BackendResult, PlaygroundResult, SubmitStatus},
        judger::*,
    },
    init::db::DB,
};

use self::{
    pubsub::{PubGuard, PubSub},
    route::*,
};
use super::code::Code;
use entity::*;

struct Waker;

impl std::task::Wake for Waker {
    fn wake(self: Arc<Self>) {
        log::error!("waker wake");
    }
}

macro_rules! check_rate_limit {
    ($s:expr) => {{
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
    JudgerUnavailable,
    #[error("`{0}`")]
    JudgerGrpc(#[from] Status),
    #[error("payload.`{0}` is not a vaild argument")]
    BadArgument(&'static str),
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

impl From<Error> for Status {
    fn from(value: Error) -> Self {
        match value {
            Error::JudgerUnavailable => Status::resource_exhausted("no available judger"),
            Error::BadArgument(x) => Status::invalid_argument(format!("bad argument: {}", x)),
            Error::JudgerGrpc(x) => report_internal!(info, "`{}`", x),
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

struct MeterGuard<'a>(&'a JudgerController);

impl<'a> Drop for MeterGuard<'a> {
    fn drop(&mut self) {
        let (num, meter) = &self.0.running_meter;
        meter.observe(num.fetch_sub(1, Ordering::Acquire) - 1, &[]);
    }
}

pub struct JudgerController {
    router: Arc<Router>,
    pubsub: Arc<PubSub<Result<SubmitStatus, Status>, i32>>,
    limiter: Arc<RateLimiter>,
    running_meter: (AtomicI64, ObservableGauge<i64>),
}

impl JudgerController {
    #[tracing::instrument(parent=span, name="judger_construct",level = "info",skip_all)]
    pub async fn new(config: Vec<config::Judger>, span: &Span) -> Result<Self, Error> {
        let router = Router::new(config, span).await?;
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
        })
    }
    fn record(&self) -> MeterGuard {
        let (num, meter) = &self.running_meter;
        meter.observe(num.fetch_add(1, Ordering::Acquire) + 1, &[]);

        MeterGuard(self)
    }
    #[instrument(skip(self,ps_guard, stream, model, scores))]
    async fn stream(
        self:Arc<Self>,
        ps_guard: PubGuard<Result<SubmitStatus, Status>, i32>,
        mut stream: tonic::Streaming<JudgeResponse>,
        mut model: submit::ActiveModel,
        mut scores: Vec<u32>,
        submit_id: i32,
    ) {
        let _=self.record();
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
                    if ps_guard.send(Ok(case.into())).is_err() {
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
        model.status = ActiveValue::Set(Some(Into::<Code>::into(status) as u32));
        model.pass_case = ActiveValue::Set(running_case);
        model.time = ActiveValue::Set(Some(time.try_into().unwrap_or(i64::MAX)));
        model.memory = ActiveValue::Set(Some(mem.try_into().unwrap_or(i64::MAX)));

        if let Err(err) = model.update(DB.get().unwrap()).await {
            log::warn!("failed to commit the judge result: {}", err);
        }
    }
    pub async fn submit(self:&Arc<Self>, submit: Submit) -> Result<i32, Error> {
        check_rate_limit!(self);
        let db = DB.get().unwrap();

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

        let mut conn = self.router.get(&submit.lang).await?;

        let res = conn
            .judge(JudgeRequest {
                lang_uid: submit.lang.to_string(),
                code: submit.code,
                memory: submit.memory_limit as u64,
                time: submit.time_limit as u64,
                rule: problem.match_rule,
                tests,
            })
            .await?;

        conn.report_success();

        tokio::spawn(self.clone().stream(
            tx,
            res.into_inner(),
            submit_model,
            scores,
            submit_id,
        ));

        Ok(submit_id)
    }
    pub async fn follow(&self, submit_id: i32) -> Option<TonicStream<SubmitStatus>> {
        self.pubsub.subscribe(&submit_id)
    }
    pub fn list_lang(&self) -> Vec<LangInfo> {
        self.router.langs.iter().map(|x| x.clone()).collect()
    }
    pub async fn playground(&self) -> Result<TonicStream<PlaygroundResult>, Error> {
        check_rate_limit!(self);

        todo!()
    }
}
