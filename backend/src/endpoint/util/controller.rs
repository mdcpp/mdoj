use tracing::{instrument, Level};

use crate::server::Server;

use super::auth::Auth;

impl Server {
    #[instrument(skip_all, level = "debug")]
    pub async fn parse_request<T: Send>(
        &self,
        request: tonic::Request<T>,
    ) -> Result<(Auth, T), tonic::Status> {
        if let Some(addr) = request.remote_addr() {
            tracing::event!(Level::DEBUG, addr = addr.to_string());
        }
        let (meta, _, payload) = request.into_parts();

        if let Some(x) = meta.get("token") {
            let token = x.to_str().unwrap();

            let user = self.token.verify(token).await?;

            tracing::event!(Level::DEBUG, user_id = user.0);

            Ok((Auth::User(user), payload))
        } else {
            tracing::trace!("token not found in metadata");
            Ok((Auth::Guest, payload))
        }
    }
}
