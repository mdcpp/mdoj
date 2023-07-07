use std::{collections::BTreeMap, pin::Pin, sync::Arc};

use futures::Stream;
use sysinfo::{CpuExt, CpuRefreshKind, System, SystemExt};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};

use super::{
    judge::{Judger, Limit},
    proto::prelude::{judge_service_server::JudgeService, *},
    spec::LangSpec,
};

use crate::{init::config::CONFIG, plugin::judge::CheckMethod};

pub struct LangJudger(Arc<LangJudgerInner>);

pub struct LangJudgerInner {
    judger: Judger,
    langs: BTreeMap<String, LangSpec>,
}

impl LangJudger {
    pub async fn new() -> Self {
        let config = CONFIG.get().unwrap();
        let plugins = match LangSpec::from_root(config.plugin.path.clone()).await {
            Ok(x) => x,
            Err(err) => {
                log::error!("Error initializing server {}", err);
                panic!("Fatal error");
            }
        };
        LangJudger(Arc::new(LangJudgerInner {
            judger: Judger::new(&config.runtime.temp),
            langs: plugins,
        }))
    }
}

macro_rules! report {
    ($tx:ident,$e:expr) => {
        if $tx.send($e).await.is_err() {
            log::warn!("gRPC client close stream before task");
        };
    };
}

macro_rules! report_status {
    ($e:expr,$tx:ident) => {
        match $e {
            Ok(x) => x,
            Err(x) => {
                let res = match x {
                    ImpossibleResource => {
                        Err(Status::resource_exhausted("Cannot preserve enough memory"))
                    }
                    Panic => Err(Status::data_loss("")),
                    _ => Ok(JudgeResponse {
                        status: x as i32,
                        time: None,
                        task: None,
                    }),
                };
                report!($tx, res);
                return ();
            }
        }
    };
}

type RunStream = Pin<Box<dyn Stream<Item = Result<JudgeResponse, Status>> + Send>>;

#[async_trait::async_trait]
impl JudgeService for LangJudger {
    async fn langs(&self, _: Request<LangRequest>) -> Result<Response<LangDescription>, Status> {
        log::trace!("Printing a list of plugins");
        let mut response = Vec::new();

        for (uuid, plugin) in &self.0.langs {
            response.push(Plugin {
                extension: plugin.extension.to_owned(),
                description: plugin.description.to_owned(),
                uuid: uuid.to_owned(),
            });
        }
        Ok(Response::new(LangDescription { plugins: response }))
    }

    type RunStream = RunStream;

    async fn run(
        &self,
        request: Request<JudgeRequest>,
    ) -> Result<Response<Self::RunStream>, Status> {
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
                    .langs
                    .get(&request.uuid)
                    .ok_or(super::JudgeStatus::NotFound),
                tx
            );

            report!(
                tx,
                Ok(JudgeResponse {
                    status: JudgeStatus::Compiling as i32,
                    task: None,
                    time: None,
                })
            );

            let judge = report_status!(inner.judger.create(spec, limit).await, tx);

            report_status!(judge.compile(request.source).await, tx);

            let mut i = 0;

            for task in request.tasks {
                i += 1;

                report!(
                    tx,
                    Ok(JudgeResponse {
                        status: JudgeStatus::Running as i32,
                        task: Some(i),
                        time: None,
                    })
                );

                let result = report_status!(judge.execute_task(task.input).await, tx);

                let method: CheckMethod = match task.method {
                    0 => CheckMethod::ExactSame,
                    1 => CheckMethod::ExactNewline,
                    2 => CheckMethod::SpaceOrNewline,
                    _ => CheckMethod::SpaceNewline,
                };

                let time = Some(result.cpu_usage.total_us);

                match result.check(task.output, method) {
                    true => {
                        report!(
                            tx,
                            Ok(JudgeResponse {
                                status: JudgeStatus::Accepted as i32,
                                task: Some(i),
                                time,
                            })
                        );
                    }
                    false => {
                        report!(
                            tx,
                            Ok(JudgeResponse {
                                status: JudgeStatus::WrongAnswer as i32,
                                task: Some(i),
                                time,
                            })
                        );
                        break;
                    }
                };
            }
        });

        Ok(Response::new(
            Box::pin(ReceiverStream::new(rx)) as Self::RunStream
        ))
    }

    async fn usage(&self, _: Request<UsageRequest>) -> Result<Response<JudgerUsage>, Status> {
        log::trace!("Retrieving plugin server usage");

        let mut sys = System::new();
        sys.refresh_cpu_specifics(CpuRefreshKind::new().with_cpu_usage());

        // let judger = self.0.judger.usage();
        todo!()
        // Ok(Response::new(JudgerUsage {
        //     all_available_mem: judger.all_available_mem,
        //     available_mem: judger.available_mem,
        //     running_task: judger.tasks,
        //     all_available_cpu_usage: sys.cpus().len() as f32,
        //     cpu_usage: sys.global_cpu_info().cpu_usage(),
        // }))
    }
}
