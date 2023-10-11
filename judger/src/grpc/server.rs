use std::{pin::Pin, sync::Arc};

use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Response, Status};

use crate::{
    grpc::proto::prelude::judge_response,
    init::config::CONFIG,
    langs::{prelude::Error as LangError, prelude::*},
};

use super::proto::prelude::{judger_server::Judger, *};

pub type UUID = String;
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

// TODO: fix bad request
macro_rules! report {
    ($result:expr,$tx:expr) => {
        match $result {
            Ok(x) => x,
            Err(err) => match err {
                LangError::Internal(err) => {
                    log::warn!("{}", err);
                    $tx.send(Err(Status::internal("Internal Error: see log.")))
                        .await
                        .ok();
                    return ();
                }
                LangError::BadRequest(err) => {
                    match err {
                        RequestError::LangNotFound => $tx
                            .send(Err(Status::invalid_argument("language uuid not found.")))
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

#[async_trait::async_trait]
impl Judger for GRpcServer {
    type JudgeStream = Pin<Box<dyn futures::Stream<Item = Result<JudgeResponse, Status>> + Send>>;

    async fn judge<'a>(
        &'a self,
        request: tonic::Request<JudgeRequest>,
    ) -> Result<Response<Self::JudgeStream>, Status> {
        let request = request.into_inner();

        let (tx, rx) = mpsc::channel(2);

        let factory = self.factory.clone();

        // precondidtion
        let mode = JudgeMatchRule::from_i32(request.rule)
            .ok_or(Status::invalid_argument("Invaild judge matching rule"))?;

        tokio::spawn(async move {
            let time = request.time;
            let memory = request.memory;

            let mut compiled = report!(factory.compile(&request.lang_uid, &request.code).await, tx);
            for task in request.tests {
                let result = report!(compiled.judge(&task.input, time, memory).await, tx);

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
        _: tonic::Request<()>,
    ) -> Result<Response<JudgeInfo>, Status> {
        log::trace!("Query judger info");
        let config = CONFIG.get().unwrap();

        let modules = self.factory.list_module();

        Ok(Response::new(JudgeInfo {
            langs: Some(Langs { list: modules }),
            memory: config.platform.available_memory,
            accuracy: (1000 * 1000 / config.kernel.USER_HZ) as i64,
            cpu_factor: config.platform.cpu_time_multiplier as f32,
        }))
    }
}
