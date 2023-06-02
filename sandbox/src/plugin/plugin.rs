use std::{collections::BTreeMap, pin::Pin, sync::Arc};

use futures::Stream;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};

use super::{
    judge::{Judger, Limit},
    proto::prelude::*,
    spec::PluginSpec,
};
use plugin_provider_server::PluginProvider;

use crate::{init::config::CONFIG, plugin::judge::CheckMethod};

pub struct PluginServer(Arc<PluginInner>);

pub struct PluginInner {
    judger: Judger,
    plugins: BTreeMap<String, PluginSpec>,
}

impl PluginServer {
    pub async fn new() -> Self {
        let config = CONFIG.get().unwrap();
        let plugins = match PluginSpec::from_root(config.plugin.path.clone()).await {
            Ok(x) => x,
            Err(err) => {
                log::error!("Error initializing server {}", err);
                panic!("Fatal error");
            }
        };
        PluginServer(Arc::new(PluginInner {
            judger: Judger::new(&config.runtime.temp),
            plugins,
        }))
    }
}

macro_rules! report {
    ($tx:ident,$e:expr) => {
        if $tx.send(Result::<_, Status>::Ok($e)).await.is_err() {
            log::warn!("gRPC client close stream before task");
        };
    };
}

macro_rules! report_status {
    ($e:expr,$tx:ident) => {
        match $e {
            Ok(x) => x,
            Err(x) => {
                report!(
                    $tx,
                    JudgeResponse {
                        status: x as i32,
                        time: None,
                        task: None,
                    }
                );
                return ();
            }
        }
    };
}

type JudgeStream = Pin<Box<dyn Stream<Item = Result<JudgeResponse, Status>> + Send>>;

#[async_trait::async_trait]
impl PluginProvider for PluginServer {
    async fn list(&self, _: Request<ListRequest>) -> Result<Response<ListResponse>, Status> {
        log::trace!("Printing a list of plugins");
        let mut response = Vec::new();

        for (uuid, plugin) in &self.0.plugins {
            response.push(Plugin {
                extension: plugin.extension.to_owned(),
                description: plugin.description.to_owned(),
                uuid: uuid.to_owned(),
            });
        }
        Ok(Response::new(ListResponse { plugins: response }))
    }

    type JudgeStream = JudgeStream;

    async fn judge(
        &self,
        request: Request<JudgeRequest>,
    ) -> Result<Response<Self::JudgeStream>, Status> {
        log::trace!("Running judge");
        let request = request.into_inner();

        let (tx, rx) = mpsc::channel(2);

        let inner = self.0.clone();

        let limit = Limit {
            cpu_us: request.cpu_us,
            mem: request.memory,
        };

        tokio::spawn(async move {
            let spec = report_status!(
                inner
                    .plugins
                    .get(&request.uuid)
                    .ok_or(super::JudgeStatus::NotFound),
                tx
            );

            report!(
                tx,
                JudgeResponse {
                    status: JudgeStatus::Compiling as i32,
                    task: None,
                    time: None,
                }
            );

            let judge = report_status!(inner.judger.create(spec, limit).await, tx);

            report_status!(judge.compile(request.source).await, tx);

            let mut i = 0;

            for task in request.tasks {
                i += 1;

                report!(
                    tx,
                    JudgeResponse {
                        status: JudgeStatus::Running as i32,
                        task: Some(i),
                        time: None,
                    }
                );

                let checker = report_status!(judge.execute_task(task.input).await, tx);

                let method: CheckMethod = match task.method {
                    0 => CheckMethod::ExactSame,
                    1 => CheckMethod::ExactNewline,
                    2 => CheckMethod::SpaceOrNewline,
                    _ => CheckMethod::SpaceNewline,
                };

                let time = Some(checker.cpu_usage.total_us);

                match checker.check(task.output, method) {
                    true => {
                        report!(
                            tx,
                            JudgeResponse {
                                status: JudgeStatus::Accepted as i32,
                                task: Some(i),
                                time,
                            }
                        );
                    }
                    false => {
                        report!(
                            tx,
                            JudgeResponse {
                                status: JudgeStatus::WrongAnswer as i32,
                                task: Some(i),
                                time,
                            }
                        );
                        break;
                    }
                };
            }
        });

        Ok(Response::new(
            Box::pin(ReceiverStream::new(rx)) as Self::JudgeStream
        ))
    }

    async fn load(&self, _: Request<LoadRequest>) -> Result<Response<LoadResponse>, Status> {
        log::trace!("Retrieving plugin server usage");
        let usage = self.0.judger.usage();
        Ok(Response::new(LoadResponse {
            all_available_mem: usage.all_available_mem,
            available_mem: usage.available_mem,
            running_task: usage.tasks,
        }))
    }
}
