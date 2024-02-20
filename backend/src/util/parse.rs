use std::num::NonZeroU32;

use tracing::instrument;

use crate::{
    controller::rate_limit::{Bucket, TrafficType},
    server::Server,
};

use super::auth::Auth;

impl Server {
    #[instrument(skip_all, level = "debug")]
    pub async fn parse_auth<T>(
        &self,
        req: &tonic::Request<T>,
    ) -> Result<(Auth, Bucket), tonic::Status> {
        let mut auth = Auth::Guest;

        let bucket = self
            .rate_limit
            .check(req, |req| async {
                if let Some(x) = req.metadata().get("token") {
                    let token = x.to_str().unwrap();

                    match self.token.verify(token).await {
                        Ok(user) => {
                            tracing::debug!(user_id = user.0);
                            auth = Auth::User(user);
                            TrafficType::Login(user.0)
                        }
                        Err(err) => {
                            tracing::debug!(msg = err.to_string());
                            TrafficType::Blacklist(err)
                        }
                    }
                } else {
                    tracing::debug!("token_missing");
                    TrafficType::Guest
                }
            })
            .await?;
        Ok((auth, bucket))
    }
    /// parse request to get bucket and payload
    ///
    #[inline]
    pub async fn parse_request<T: Send>(
        &self,
        req: tonic::Request<T>,
    ) -> Result<(Auth, Bucket, T), tonic::Status> {
        let (auth, bucket) = self.parse_auth(&req).await?;

        Ok((auth, bucket, req.into_inner()))
    }
    /// parse request to get bucket and payload
    /// and immediately rate limiting
    #[inline]
    pub async fn parse_request_n<T>(
        &self,
        req: tonic::Request<T>,
        permit: NonZeroU32,
    ) -> Result<(Auth, T), tonic::Status> {
        let (auth, bucket) = self.parse_auth(&req).await?;

        bucket.cost(permit)?;

        Ok((auth, req.into_inner()))
    }
}
