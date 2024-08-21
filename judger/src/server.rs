use std::{pin::Pin, str::FromStr, sync::Arc};

use async_stream::try_stream;
use futures_core::Stream;
use grpc::judger::{judger_server::*, *};
use tokio::{fs::File, sync::Semaphore};
use tokio_stream::StreamExt;
use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::{
    error::{ClientError, Error},
    language::{ExecuteArgBuilder, JudgeArgBuilder, PluginMap},
    CONFIG,
};

const PLUGIN_PATH: &str = "plugins";

fn check_secret<T>(req: Request<T>) -> Result<T, Status> {
    let (meta, _, payload) = req.into_parts();
    if CONFIG.secret.is_none() {
        return Ok(payload);
    }
    let secret = CONFIG.secret.as_ref().unwrap();
    if let Some(header) = meta.get("Authorization") {
        let secret = ["basic ", secret].concat().into_bytes();
        let valid = header
            .as_bytes()
            .iter()
            .zip(secret.iter())
            .map(|(&a, &b)| a == b)
            .reduce(|a, b| a && b);
        if valid.unwrap_or(false) {
            return Ok(payload);
        }
    }
    Err(Status::permission_denied("Invalid secret"))
}

pub struct Server {
    semaphore: Arc<Semaphore>,
    plugins: PluginMap<File>,
}

impl Server {
    pub async fn new() -> crate::Result<Server> {
        let semaphore = Arc::new(Semaphore::new(CONFIG.memory.try_into().unwrap()));
        let plugins = PluginMap::new(PLUGIN_PATH).await?;
        Ok(Server { semaphore, plugins })
    }
}

#[tonic::async_trait]
impl Judger for Server {
    type JudgeStream = Pin<Box<dyn Stream<Item = Result<JudgeResponse, Status>> + Send>>;

    async fn judge(
        &self,
        req: Request<JudgeRequest>,
    ) -> Result<Response<Self::JudgeStream>, Status> {
        let payload = check_secret(req)?;

        let memory = payload.memory;
        let cpu = payload.time;
        let source = payload.code;
        let uuid =
            Uuid::from_str(&payload.lang_uid).map_err(|_| ClientError::InvalidLanguageUuid)?;

        let plugin = self
            .plugins
            .get(&uuid)
            .ok_or(ClientError::InvalidLanguageUuid)?;
        let resource: u32 = plugin
            .get_memory_reserved(payload.memory)
            .try_into()
            .map_err(|_| Error::Platform)?;
        let permit = self
            .semaphore
            .clone()
            .acquire_many_owned(resource)
            .await
            .map_err(|_| ClientError::ImpossibleMemoryRequirement)?;

        let (input, output): (Vec<Vec<u8>>, Vec<Vec<u8>>) = payload
            .tests
            .into_iter()
            .map(|x| (x.input, x.output))
            .unzip();

        let args = JudgeArgBuilder::new()
            .cpu(cpu)
            .mem(memory)
            .input(input.into_iter())
            .output(output.into_iter())
            .mode(payload.rule.into())
            .source(source)
            .build();

        let mut result = plugin.judge(args).await;

        Ok(Response::new(Box::pin(try_stream! {
            while let Some(r) = result.next().await {
                yield JudgeResponse::from(r?);
            }
            drop(permit);
        })))
    }

    async fn judger_info(&self, req: Request<()>) -> Result<Response<JudgeInfo>, Status> {
        check_secret(req)?;
        let list = self
            .plugins
            .iter()
            .map(|v| v.get_info().clone())
            .collect::<Vec<_>>();
        Ok(Response::new(JudgeInfo {
            memory: CONFIG.memory,
            accuracy: 0, // FIXME: accuracy
            langs: Langs { list },
            cpu_factor: CONFIG.ratio.cpu as f32,
        }))
    }

    type ExecStream = tokio_stream::Once<Result<ExecResult, Status>>;

    async fn exec(&self, req: Request<ExecRequest>) -> Result<Response<Self::ExecStream>, Status> {
        let payload = check_secret(req)?;

        let memory = payload.memory;
        let cpu = payload.time;

        let source = payload.code;
        let input = payload.input;

        let uuid =
            Uuid::from_str(&payload.lang_uid).map_err(|_| ClientError::InvalidLanguageUuid)?;

        let plugin = self
            .plugins
            .get(&uuid)
            .ok_or(ClientError::InvalidLanguageUuid)?;

        let resource: u32 = plugin
            .get_memory_reserved(payload.memory)
            .try_into()
            .map_err(|_| Error::Platform)?;

        let permit = self
            .semaphore
            .clone()
            .acquire_many_owned(resource)
            .await
            .map_err(|_| ClientError::ImpossibleMemoryRequirement)?;

        let args = ExecuteArgBuilder::new()
            .cpu(cpu)
            .mem(memory)
            .source(source)
            .input(input)
            .build();

        let result = plugin.execute(args).await?;
        drop(permit);
        Ok(Response::new(tokio_stream::once(Ok(result.into()))))
    }
}
