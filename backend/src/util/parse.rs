use tracing::{instrument, Level};

use crate::server::Server;

use super::auth::Auth;

impl Server {
    /// parse request
    #[instrument(skip_all, level = "debug")]
    pub async fn parse_request<T: Send>(
        &self,
        req: tonic::Request<T>,
    ) -> Result<(Auth, T), tonic::Status> {
        self.rate_limit.check_ip(&req, 1)?;
        let (meta, _, payload) = req.into_parts();

        if let Some(x) = meta.get("token") {
            let token = x.to_str().unwrap();

            let user = self.token.verify(token).await?;

            tracing::event!(Level::DEBUG, user_id = user.0);

            Ok((Auth::User(user), payload))
        } else {
            tracing::trace!("token_missing");
            Ok((Auth::Guest, payload))
        }
    }
}
