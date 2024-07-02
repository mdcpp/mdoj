use super::tools::*;

use grpc::backend::imgur_set_server::*;
use grpc::backend::*;

#[async_trait]
impl ImgurSet for ArcServer {
    #[instrument(skip_all, level = "debug")]
    async fn upload(
        &self,
        req: Request<UploadRequest>,
    ) -> Result<Response<UploadResponse>, Status> {
        let (auth, req) = self.parse_request_n(req, crate::NonZeroU32!(5)).await?;
        let (user_id, _) = auth.ok_or_default()?;

        let uuid = Uuid::parse_str(&req.request_id).map_err(Error::InvaildUUID)?;
        if let Some(x) = self.dup.check::<UploadResponse>(user_id, uuid) {
            return Ok(Response::new(x));
        };

        let url = self.imgur.upload(req.data).await?;

        tracing::debug!(request_id = uuid.to_string(), uri = url, "image_uploaded");
        let url = UploadResponse { url };

        self.dup.store(user_id, uuid, url.clone());
        self.metrics.image(1);

        Ok(Response::new(url))
    }
}
