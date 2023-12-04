use tracing::{span, Level};

use crate::server::Server;

use super::auth::Auth;

impl Server {
    pub async fn parse_request<T: Send>(
        &self,
        request: tonic::Request<T>,
    ) -> Result<(Auth, T), tonic::Status> {
        let span = span!(Level::INFO,"token_verify",addr=?request.remote_addr());
        let _ = span.enter();

        let (meta, _, payload) = request.into_parts();

        if let Some(x) = meta.get("token") {
            let token = x.to_str().unwrap();

            Ok((Auth::User(self.token.verify(token).await?), payload))
        } else {
            Ok((Auth::Guest, payload))
        }
    }
}
