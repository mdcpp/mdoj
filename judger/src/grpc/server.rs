use std::{pin::Pin, sync::Arc};

use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{codegen::Bytes, metadata, Code, Response, Status};

use crate::{
    grpc::proto::prelude::judge_response,
    init::config::CONFIG,
    langs::{prelude::Error as LangError, prelude::*},
};

use super::proto::prelude::{judger_server::Judger, *};

pub type UUID = String;

macro_rules! report {
    ($result:expr,$tx:expr) => {
        match $result {
            Ok(x) => x,
            Err(err) => match err {
                LangError::Internal(err) => {
                    log::warn!("{}", err);
                    #[cfg(debug_assertions)]
                    $tx.send(Err(Status::with_details(
                        Code::Internal,
                        "Lanuage internal error: see debug info",
                        Bytes::from(format!("{}", err)),
                    )))
                    .await
                    .ok();
                    #[cfg(not(debug_assertions))]
                    $tx.send(Err(Status::internal("See log for more details")))
                        .await
                        .ok();
                    return ();
                }
                LangError::BadRequest(err) => {
                    match err {
                        RequestError::LangNotFound(uid) => $tx
                            .send(Err(Status::with_details(
                                Code::FailedPrecondition,
                                "language with such uuid does not exist on this judger",
                                Bytes::from(format!("lang_uid: {}", uid)),
                            )))
                            .await
                            .ok(),
                    };
                    return ();
                }
                LangError::Report(res) => {
                    $tx.send(Ok(JudgeResponse {
                        task: Some(judge_response::Task::Result(JudgeResult {
                            status: res as i32,
                            max_time: None,
                            max_mem: None,
                        })),
                    }))
                    .await
                    .ok();
                    return ();
                }
            },
        }
    };
}
pub struct GRpcServer {
    factory: Arc<ArtifactFactory>,
}

impl GRpcServer {
    pub async fn new() -> Self {
        let config = CONFIG.get().unwrap();
        let mut factory = ArtifactFactory::default();

        factory.load_dir(config.plugin.path.clone()).await;

        Self {
            factory: Arc::new(factory),
        }
    }
}

#[tonic::async_trait]
impl Judger for GRpcServer {
    type JudgeStream = Pin<Box<dyn futures::Stream<Item = Result<JudgeResponse, Status>> + Send>>;

    async fn judge<'a>(
        &'a self,
        request: tonic::Request<JudgeRequest>,
    ) -> Result<Response<Self::JudgeStream>, Status> {
        let (meta, _, request) = request.into_parts();
        check_secret(&meta)?;

        let (tx, rx) = mpsc::channel(2);

        let factory = self.factory.clone();

        // precondidtion
        let mode = JudgeMatchRule::from_i32(request.rule)
            .ok_or(Status::invalid_argument("Invaild judge matching rule"))?;

        tokio::spawn(async move {
            let time = request.time;
            let memory = request.memory;

            let mut compiled = report!(factory.compile(&request.lang_uid, &request.code).await, tx);

            let mut running_task = 1;

            for task in request.tests {
                tx.send(Ok(JudgeResponse {
                    task: Some(judge_response::Task::Case(running_task)),
                }))
                .await
                .ok();

                running_task += 1;

                let result = report!(compiled.judge(&task.input, time, memory).await, tx);

                if let Some(x) = result.get_expection() {
                    tx.send(Ok(JudgeResponse {
                        task: Some(judge_response::Task::Result(JudgeResult {
                            status: x as i32,
                            max_time: None,
                            max_mem: None,
                        })),
                    }))
                    .await
                    .ok();
                    return;
                }

                tx.send(Ok(JudgeResponse {
                    task: Some(judge_response::Task::Result(JudgeResult {
                        status: match result.assert(&task.output, mode) {
                            true => JudgeResultState::Ac,
                            false => JudgeResultState::Wa,
                        } as i32,
                        max_time: Some(result.time().total_us),
                        max_mem: Some(result.mem().peak),
                    })),
                }))
                .await
                .ok();
            }
        });

        Ok(Response::new(Box::pin(ReceiverStream::new(rx))))
        }
    async fn judger_info<'a>(
        &'a self,
        request: tonic::Request<()>,
    ) -> Result<Response<JudgeInfo>, Status> {
        log::trace!("Query judger info");
        let config = CONFIG.get().unwrap();

        let (meta, _, _) = request.into_parts();
        check_secret(&meta)?;

        let modules = self.factory.list_module();

        Ok(Response::new(JudgeInfo {
            langs: Some(Langs { list: modules }),
            memory: config.platform.available_memory,
            accuracy: (1000 * 1000 / config.kernel.kernel_hz) as u64,
            cpu_factor: config.platform.cpu_time_multiplier as f32,
        }))
    }
}

fn check_secret(meta: &metadata::MetadataMap) -> Result<(), Status> {
    let config = CONFIG.get().unwrap();
    if config.secret.is_none() {
        return Ok(());
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
            return Ok(());
        }
    }
    Err(Status::permission_denied("Invalid secret"))
}
