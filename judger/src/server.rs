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
    language::{JudgeArgBuilder, Map},
    CONFIG,
};

fn check_secret<T>(req: tonic::Request<T>) -> Result<T, Status> {
    let (meta, _, payload) = req.into_parts();
    if CONFIG.secret.is_none() {
        return Ok(payload);
    }
    let secret = CONFIG.secret.as_ref().unwrap();
    if let Some(header) = meta.get("Authorization") {
        let secret = ["basic ", secret].concat().into_bytes();
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

pub struct Server {
    semaphore: Arc<Semaphore>,
    plugins: Map<File>,
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
            Uuid::from_str(&payload.lang_uid).map_err(|_| ClientError::InvaildLanguageUuid)?;

        let plugin = self
            .plugins
            .get(&uuid)
            .ok_or(ClientError::InvaildLanguageUuid)?;
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

    async fn judger_info(&self, req: tonic::Request<()>) -> Result<Response<JudgeInfo>, Status> {
        todo!()
    }

    type ExecStream = tokio_stream::Iter<std::vec::IntoIter<Result<ExecResult, Status>>>;

    async fn exec(
        &self,
        req: Request<ExecRequest>,
    ) -> Result<Response<Self::ExecStream>, tonic::Status> {
        todo!()
    }
}
