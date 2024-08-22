use super::*;

use grpc::backend::image_server::*;

#[async_trait]
impl Image for ArcServer {
    #[instrument(
        skip_all,
        level = "info",
        name = "oj.backend.Image/upload",
        err(level = "debug", Display)
    )]
    async fn upload(
        &self,
        req: Request<UploadRequest>,
    ) -> Result<Response<UploadResponse>, Status> {
        let (auth, req) = self.rate_limit(req).in_current_span().await?;
        auth.assume_login()?;
        req.get_or_insert(|req| async move {
            let url = self.imgur.upload(req.data).await?;

            debug!(counter.image = 1, uri = url);
            Ok(UploadResponse { url })
        })
        .await
        .with_grpc()
        .into()
    }
}
