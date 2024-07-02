use super::tools::*;

use crate::controller::judger::PlaygroundPayload;

use grpc::backend::playground_set_server::*;
use grpc::backend::*;

const PLAYGROUND_CODE_LEN: usize = 32 * 1024;

#[async_trait]
impl PlaygroundSet for ArcServer {
    #[doc = " Server streaming response type for the Run method."]
    type RunStream = TonicStream<PlaygroundResult>;

    #[instrument(skip_all, level = "debug")]
    async fn run(
        &self,
        req: Request<PlaygroundRequest>,
    ) -> Result<Response<Self::RunStream>, Status> {
        let (auth, req) = self
            .parse_request_n(req, crate::NonZeroU32!(15))
            .in_current_span()
            .await?;
        let (user_id, _) = auth.ok_or_default()?;

        tracing::debug!(user_id = user_id);

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
                .instrument(tracing::debug_span!("playground"))
                .await?,
        ))
    }
}
