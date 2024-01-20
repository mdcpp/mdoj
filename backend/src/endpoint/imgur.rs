use super::tools::*;

use crate::grpc::backend::imgur_set_server::*;
use crate::grpc::backend::*;

#[async_trait]
impl ImgurSet for Arc<Server> {
    #[instrument(skip_all, level = "debug")]
    async fn upload(
        &self,
        req: Request<UploadRequest>,
    ) -> Result<Response<UploadResponse>, Status> {
        let (auth, req) = self.parse_request(req).await?;
        let (user_id, perm) = auth.ok_or_default()?;

        // if (!perm.can_imgur()) & (!perm.can_root()) {
        //     return Err(Error::RequirePermission("image").into());
        // }

        let uuid = Uuid::parse_str(&req.request_id).map_err(Error::InvaildUUID)?;

        if let Some(x) = self.dup.check_str(user_id, &uuid) {
            return Ok(Response::new(UploadResponse { url: x.to_owned() }));
        };

        let url = self.imgur.upload(req.data).await?;

        self.dup.store_str(user_id, uuid, url.to_owned());

        tracing::debug!(request_id = uuid.to_string(), uri = url, "image_uploaded");

        self.metrics.image.observe(1, &[]);

        Ok(Response::new(UploadResponse { url }))
    }
}
