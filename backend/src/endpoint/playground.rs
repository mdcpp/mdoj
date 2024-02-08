use super::tools::*;

use crate::controller::judger::PlaygroundPayload;
use crate::grpc::backend;
use crate::grpc::backend::playground_set_server::*;
use crate::grpc::backend::*;
use crate::grpc::judger;
use crate::grpc::judger::exec_result;
use crate::grpc::judger::ExecResult;

const PLAYGROUND_CODE_LEN: usize = 32 * 1024;

#[async_trait]
impl PlaygroundSet for Arc<Server> {
    #[doc = " Server streaming response type for the Run method."]
    type RunStream = TonicStream<PlaygroundResult>;

    #[instrument(skip_all, level = "debug")]
    async fn run(
        &self,
        req: Request<PlaygroundRequest>,
    ) -> Result<Response<Self::RunStream>, Status> {
        let (auth, req) = self.parse_request_n(req, crate::NonZeroU32!(15)).await?;
        let (user_id, _) = auth.ok_or_default()?;

        tracing::debug!(user_id = user_id, "playground_start");

        if req.code.len() > PLAYGROUND_CODE_LEN {
            return Err(Error::BufferTooLarge("code").into());
        }

        let lang = Uuid::parse_str(&req.lang).map_err(Error::InvaildUUID)?;

        Ok(Response::new(
            self.judger
                .playground(PlaygroundPayload {
                    input: req.input,
                    code: req.code,
                    lang,
                })
                .await?,
        ))
    }
}

impl From<ExecResult> for PlaygroundResult {
    fn from(value: ExecResult) -> Self {
        PlaygroundResult {
            result: Some(match value.result.unwrap() {
                exec_result::Result::Output(x) => playground_result::Result::Output(x),
                exec_result::Result::Log(x) => playground_result::Result::Compile(x.into()),
            }),
        }
    }
}
impl From<judger::Log> for backend::Log {
    fn from(value: judger::Log) -> Self {
        backend::Log {
            level: value.level,
            msg: value.msg,
        }
    }
}
