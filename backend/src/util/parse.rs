use tracing::instrument;

use crate::{controller::rate_limit::TrafficType, server::Server};

use super::auth::Auth;

impl Server {
    pub async fn parse_auth<T>(&self, req: &tonic::Request<T>) -> Result<Auth, tonic::Status> {
        let mut auth = Auth::Guest;

        self.rate_limit
            .check(&req, |req| async {
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
    #[instrument(skip_all, level = "debug")]
    pub async fn parse_request<T: Send>(
        &self,
        req: tonic::Request<T>,
    ) -> Result<(Auth, T), tonic::Status> {
        let mut auth = Auth::Guest;

        self.rate_limit
            .check(&req, |req| async {
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

        Ok((auth, req.into_inner()))
    }
}
