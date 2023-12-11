// TODO: clean up imports
// TODO: error handling
use std::{pin::Pin, sync::Arc};

use spin::Mutex;
use tokio::sync::mpsc::*;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Response, Status};
use uuid::Uuid;

use crate::{
    grpc::proto::prelude::judge_response,
    init::config::CONFIG,
    langs::prelude::{ArtifactFactory, CompileLog},
};

use super::proto::prelude::{judger_server::Judger, *};

const PENDING_LIMIT: usize = 128;
const STREAM_CHUNK: usize = 1024 * 16;

pub type UUID = String;

fn accuracy() -> u64 {
    let config = CONFIG.get().unwrap();
    (1000 * 1000 / config.kernel.kernel_hz) as u64
}

impl From<CompileLog> for ExecResult {
    fn from(value: CompileLog) -> Self {
        ExecResult {
            result: Some(exec_result::Result::Log(Log {
                level: value.level as u32,
                msg: value.message,
            })),
        }
    }
}

fn parse_uid(uid: &str) -> Result<Uuid, Status> {
    Uuid::parse_str(uid).map_err(|e| {
        log::warn!("Invalid uuid: {}", e);
        Status::failed_precondition("Invalid uuid")
    })
}

async fn force_stream<T>(tx: &mut Sender<Result<T, Status>>, item: T) -> Result<(), Status> {
    match tx.send(Ok(item)).await {
        Ok(_) => Ok(()),
        Err(err) => {
            log::debug!("client disconnected: {}", err);
            Err(Status::cancelled("client disconnect durning operation!"))
        }
    }
}

fn check_secret<T>(req: tonic::Request<T>) -> Result<T, Status> {
    let (meta, _, payload) = req.into_parts();
    let config = CONFIG.get().unwrap();
    if config.secret.is_none() {
        return Ok(payload);
    }
    if let Some(header) = meta.get("Authorization") {
        let secret = ["basic ", config.secret.as_ref().unwrap()]
            .concat()
            .into_bytes();
        let vaild = header
            .as_bytes()
            .iter()
            .zip(secret.iter())
            .map(|(&a, &b)| a == b)
            .reduce(|a, b| a && b);
        if vaild.unwrap_or(false) {
            return Ok(payload);
        }
    }
    Err(Status::permission_denied("Invalid secret"))
}

impl From<judge_response::Task> for JudgeResponse {
    fn from(value: judge_response::Task) -> Self {
        Self { task: Some(value) }
    }
}

// Adapter and abstraction for tonic to serve
// utilize artifact factory and other components(in module `langs``)
pub struct Server {
    factory: ArtifactFactory,
    running: Mutex<usize>,
}

impl Server {
    pub async fn new() -> Self {
        let config = CONFIG.get().unwrap();
        let mut factory = ArtifactFactory::default();

        factory.load_dir(config.plugin.path.clone()).await;

        Self {
            factory: factory,
            running: Mutex::new(PENDING_LIMIT),
        }
    }
    fn check_pending(self: &Arc<Self>) -> Result<PendingGuard, Status> {
        let mut running = self.running.lock();
        if *running > 0 {
            *running -= 1;
            Ok(PendingGuard(self.clone()))
        } else {
            Err(Status::resource_exhausted(""))
        }
    }
}

struct PendingGuard(Arc<Server>);

impl Drop for PendingGuard {
    fn drop(&mut self) {
        *self.0.running.lock() -= 1;
    }
}

async fn judger_stream(
    factory: &ArtifactFactory,
    payload: JudgeRequest,
    tx: &mut Sender<Result<JudgeResponse, Status>>,
) -> Result<(), Status> {
    log::debug!("start streaming");

    let mode = JudgeMatchRule::from_i32(payload.rule)
        .ok_or(Status::failed_precondition("Invaild judge matching rule"))?;
    let lang = parse_uid(&payload.lang_uid)?;

    let mut compile = factory.compile(&lang, &payload.code).await?;

    compile.log().for_each(|x| x.log());

    if let Some(code) = compile.get_expection() {
        force_stream(
            tx,
            judge_response::Task::Result(JudgeResult {
                status: code.into(),
                ..Default::default()
            })
            .into(),
        )
        .await?;
    }

    for (running_task, test) in payload.tests.into_iter().enumerate() {
        log::trace!("running at {} task", running_task);
        force_stream(
            tx,
            JudgeResponse {
                task: Some(judge_response::Task::Case(running_task.try_into().unwrap())),
            },
        )
        .await?;

        let mut result = compile
            .judge(&test.input, payload.time, payload.memory)
            .await?;

        if let Some(code) = result.get_expection() {
            log::trace!("yield result: {}", code);
            force_stream(
                tx,
                judge_response::Task::Result(JudgeResult {
                    status: code.into(),
                    ..Default::default()
                })
                .into(),
            )
            .await?;
            break;
        }

        let code = match result.assert(&test.input, mode) {
            true => JudgerCode::Ac,
            false => JudgerCode::Wa,
        };

        let time = result.time().total_us;
        let memory = result.mem().peak;
        log::trace!(
            "yield result: {}, take memory {}B, total_us: {}ns",
            code,
            time,
            memory
        );
        force_stream(
            tx,
            judge_response::Task::Result(JudgeResult {
                status: code.into(),
                time,
                memory,
                accuracy: accuracy(),
            })
            .into(),
        )
        .await?;
    }
    Ok(())
}

async fn exec_stream(
    factory: &ArtifactFactory,
    payload: ExecRequest,
    tx: &mut Sender<Result<ExecResult, Status>>,
) -> Result<(), Status> {
    log::debug!("start streaming");

    let lang = parse_uid(&payload.lang_uid)?;

    let mut compile = factory.compile(&lang, &payload.code).await?;

    for log in compile.log() {
        force_stream(tx, log.into()).await?;
    }

    if let Some(_) = compile.get_expection() {
        force_stream(
            tx,
            CompileLog {
                level: 4,
                message: "Compile Error, non-zero return code(signal)".to_string(),
            }
            .into(),
        )
        .await?;
        return Ok(());
    }

    let mut judge = compile
        .judge(&payload.input, payload.time, payload.memory)
        .await?;

    if let Some(x) = judge.get_expection() {
        force_stream(
            tx,
            CompileLog {
                level: 4,
                message: format!("Judge Fail with {}", x),
            }
            .into(),
        )
        .await?;
    } else {
        for chunk in judge.process().unwrap().stdout.chunks(STREAM_CHUNK) {
            force_stream(
                tx,
                ExecResult {
                    result: Some(exec_result::Result::Output(chunk.to_vec())),
                },
            )
            .await?;
        }
    }

    Ok(())
}

#[tonic::async_trait]
impl Judger for Arc<Server> {
    type JudgeStream = Pin<Box<dyn futures::Stream<Item = Result<JudgeResponse, Status>> + Send>>;

    async fn judge(
        &self,
        req: tonic::Request<JudgeRequest>,
    ) -> Result<Response<Self::JudgeStream>, Status> {
        let payload = check_secret(req)?;
        let permit = self.check_pending()?;

        log::debug!("start judging");

        let (mut tx, rx) = channel(8);

        let self_ = self.clone();

        tokio::spawn(async move {
            if let Err(err) = judger_stream(&self_.factory, payload, &mut tx).await {
                tx.send(Err(err)).await.ok();
            };
            drop(permit);
        });

        Ok(Response::new(Box::pin(ReceiverStream::new(rx))))
    }
    async fn judger_info(&self, req: tonic::Request<()>) -> Result<Response<JudgeInfo>, Status> {
        let config = CONFIG.get().unwrap();
        check_secret(req)?;

        let modules = self.factory.list_module();

        Ok(Response::new(JudgeInfo {
            langs: Langs { list: modules },
            memory: config.platform.available_memory,
            accuracy: accuracy(),
            cpu_factor: config.platform.cpu_time_multiplier as f32,
        }))
    }

    #[doc = " Server streaming response type for the Exec method."]
    type ExecStream = Pin<Box<dyn futures::Stream<Item = Result<ExecResult, Status>> + Send>>;

    async fn exec(
        &self,
        req: tonic::Request<ExecRequest>,
    ) -> Result<Response<Self::ExecStream>, tonic::Status> {
        let payload = check_secret(req)?;
        let permit = self.check_pending()?;

        log::debug!("start exec");

        let (mut tx, rx) = channel(8);

        let self_ = self.clone();

        tokio::spawn(async move {
            if let Err(err) = exec_stream(&self_.factory, payload, &mut tx).await {
                tx.send(Err(err)).await.ok();
            };
            drop(permit);
        });

        Ok(Response::new(Box::pin(ReceiverStream::new(rx))))
    }
}
