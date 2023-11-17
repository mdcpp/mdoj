use tracing::{span, Level};

use crate::server::Server;

use super::{auth::Auth, error::Error};

impl Server {
    pub async fn parse_request<T: Send>(
        &self,
        request: tonic::Request<T>,
    ) -> Result<(Auth, T), Error> {
        let span = span!(Level::INFO,"token_verify",addr=?request.remote_addr());
        let _ = span.enter();

        let (meta, _, payload) = request.into_parts();

        if let Some(x) = meta.get("token") {
            let token = x.to_str().unwrap();

            match self.token.verify(token).await.map_err(|x| {
                log::error!("Token verification failed: {}", x);
                Error::Unauthenticated
            })? {
                Some(x) => Ok((Auth::User(x), payload)),
                None => Err(Error::Unauthenticated),
            }
        } else {
            Ok((Auth::Guest, payload))
        }
    }
}
