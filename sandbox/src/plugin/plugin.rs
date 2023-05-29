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
    ($e:expr,$s:expr,$tx:expr) => {
        match $e {
            Some(x) => x,
            None => {
                if $tx
                    .send(Result::<_, Status>::Ok(JudgeResponse {
                        status: $s as i32,
                        time: None,
                        finished: None,
                    }))
                    .await
                    .is_err()
                {
                    log::warn!("gRPC client close stream before finished");
                };
                return ();
            }
        }
    };
}

type JudgeStream = Pin<Box<dyn Stream<Item = Result<JudgeResponse, Status>> + Send>>;

#[async_trait::async_trait]
impl PluginProvider for PluginServer {
    async fn list(&self, _: Request<ListRequest>) -> Result<Response<ListResponse>, Status> {
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
        let request = request.into_inner();

        let (tx, rx) = mpsc::channel(2);

        let inner = self.0.clone();

        let limit = Limit {
            cpu_us: request.cpu_us,
            mem: request.memory,
        };

        tokio::spawn(async move {
            let spec = report!(inner.plugins.get(&request.uuid), JudgeStatus::NotFound, tx);

            let judge = report!(
                inner.judger.create(spec, limit).await,
                JudgeStatus::CompileError,
                tx
            );

            report!(
                judge.compile(request.source).await,
                JudgeStatus::CompileError,
                tx
            );

            let mut i = 0;

            for task in request.tasks {
                i += 1;
                let checker = report!(
                    judge.execute_task(task.input).await,
                    JudgeStatus::RuntimeError,
                    tx
                );

                let method: CheckMethod = match task.method {
                    0 => CheckMethod::ExactSame,
                    1 => CheckMethod::ExactNewline,
                    2 => CheckMethod::SpaceOrNewline,
                    _ => CheckMethod::SpaceNewline,
                };

                let time = Some(checker.cpu_usage.total_us);

                let status = match checker.check(task.output, method) {
                    true => JudgeStatus::Accepted,
                    false => JudgeStatus::WrongAnswer,
                };

                if tx
                    .send(Ok(JudgeResponse {
                        status: status as i32,
                        finished: Some(i),
                        time,
                    }))
                    .await
                    .is_err()
                {
                    log::warn!("gRPC client close stream before finished");
                    break;
                };

                if status != JudgeStatus::Accepted {
                    break;
                }
            }
        });

        Ok(Response::new(
            Box::pin(ReceiverStream::new(rx)) as Self::JudgeStream
        ))
    }
}
