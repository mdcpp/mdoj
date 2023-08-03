use std::{pin::Pin, sync::Arc};

use futures::Stream;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Response, Status};

use crate::{
    grpc::proto::prelude::judge_response::Task,
    init::config::CONFIG,
    langs::{self, prelude::ArtifactFactory},
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
                langs::Error::Internal(x) => {
                    log::warn!("{}", x);
                    $tx.send(Err(Status::internal("See Log"))).await.ok();
                    return ();
                }
                langs::Error::BadRequest(_) => {
                    $tx.send(Err(Status::internal("Bad Request"))).await.ok();
                    return ();
                }
                langs::Error::Report(x) => {
                    $tx.send(Ok(JudgeResponse {
                        task: Some(Task::Result(JudgeResult {
                            status: x as i32,
                            max_time: 0.0,
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
    type JudgeStream = Pin<Box<dyn Stream<Item = Result<JudgeResponse, Status>> + Send>>;

    async fn judge<'a>(
        &'a self,
        request: tonic::Request<JudgeRequest>,
    ) -> Result<Response<Self::JudgeStream>, Status> {
        let request = request.into_inner();

        let (tx, rx) = mpsc::channel(2);

        let factory = self.factory.clone();

        tokio::spawn(async move {
            let time = request.time;
            let memory = request.memory;
            let mode = match request.rule {
                0 => JudgeMatchRule::ExactSame,
                1 => JudgeMatchRule::IgnoreSpace,
                _ => JudgeMatchRule::SkipSnl,
            };

            let mut compiled = report!(factory.compile(&request.lang_uid, &request.code).await, tx);

            for task in request.tests {
                let result = report!(compiled.judge(&task.input, time, memory).await, tx);

                tx.send(Ok(JudgeResponse {
                    task: Some(Task::Result(JudgeResult {
                        status: match result.assert(&task.output, mode) {
                            true => JudgeResultState::Ac,
                            false => JudgeResultState::Wa,
                        } as i32,
                        max_time: 0.0,
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