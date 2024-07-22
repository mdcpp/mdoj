use super::tools::*;

use grpc::backend::playground_server::*;
use grpc::backend::*;

#[async_trait]
impl Playground for ArcServer {
    #[instrument(skip_all, level = "debug")]
    async fn list_lang(&self, req: Request<()>) -> Result<Response<Languages>, Status> {
        self.parse_request_n(req, NonZeroU32!(5))
            .in_current_span()
            .await?;

        let list: Vec<_> = self
            .judger
            .list_lang()
            .into_iter()
            .map(|x| x.into())
            .collect();

        Ok(Response::new(Languages { list }))
    }
}
