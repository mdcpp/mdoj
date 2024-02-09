use std::num::NonZeroU32;

use tracing::instrument;

use crate::{controller::rate_limit::TrafficType, server::Server};

use super::auth::Auth;

impl Server {
    #[instrument(skip_all, level = "debug")]
    pub async fn parse_auth<T>(
        &self,
        req: &tonic::Request<T>,
        permit: NonZeroU32,
    ) -> Result<Auth, tonic::Status> {
        let mut auth = Auth::Guest;

        self.rate_limit
            .check(req, permit, |req| async {
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
        Ok(auth)
    }
    /// parse request
    ///
    /// behind the sence we do rate limit(consume two resource)
    #[inline]
    pub async fn parse_request<T: Send>(
        &self,
        req: tonic::Request<T>,
    ) -> Result<(Auth, T), tonic::Status> {
        let auth = self.parse_auth(&req, crate::NonZeroU32!(2)).await?;

        Ok((auth, req.into_inner()))
    }
    /// parse request and rate limit
    #[inline]
    pub async fn parse_request_n<T>(
        &self,
        req: tonic::Request<T>,
        permit: NonZeroU32,
    ) -> Result<(Auth, T), tonic::Status> {
        let auth = self.parse_auth(&req, permit).await?;

        Ok((auth, req.into_inner()))
    }
}
