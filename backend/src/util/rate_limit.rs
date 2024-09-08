use std::num::NonZeroU32;

use super::auth::Auth;
use crate::{
    controller::rate_limit::{Bucket, TrafficType},
    server::Server,
};
use grpc::backend::{Id, *};
use tracing::*;
use tracing_futures::Instrument;

impl Server {
    /// parse authentication without rate limiting
    ///
    /// It's useful for endpoints that require resolving identity
    /// before rate limiting, such as logout
    #[instrument(skip_all, level = "info")]
    pub async fn authenticate_user<T>(
        &self,
        req: &tonic::Request<T>,
    ) -> Result<(Auth, Bucket), tonic::Status> {
        let mut auth = Auth::Guest;

        let bucket = self
            .rate_limit
            .check(req, |req| async {
                if let Some(x) = req.metadata().get("token") {
                    let token = x.to_str().unwrap();
                    tracing::debug!(token = token);

                    match self.token.verify(token).in_current_span().await {
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
                    TrafficType::Guest
                }
            })
            .in_current_span()
            .await?;
        tracing::info!(auth = %auth);
        Ok((auth, bucket))
    }
    #[instrument(skip_all, level = "info", fields(cost))]
    pub async fn rate_limit<T: RateLimit>(
        &self,
        req: tonic::Request<T>,
    ) -> Result<(Auth, T), tonic::Status> {
        let (auth, bucket) = self.authenticate_user(&req).in_current_span().await?;
        bucket.cost(NonZeroU32::new(3).unwrap())?;
        let req = req.into_inner();
        tracing::debug!(bucket = %bucket);

        if let Some(cost) = NonZeroU32::new(req.get_cost()) {
            Span::current().record("cost", cost.saturating_add(3));
            bucket.cost(cost)?;
        } else {
            Span::current().record("cost", 3);
        }

        Ok((auth, req))
    }
}

pub trait RateLimit {
    fn get_cost(&self) -> u32 {
        10
    }
}

macro_rules! impl_list_rate_limit {
    ($t:ident) => {
        paste::paste!{
            impl RateLimit for [<List $t Request>]{
                fn get_cost(&self) -> u32 {
                    self.size.saturating_add(self.offset.unsigned_abs() / 7).saturating_add(5).min(u32::MAX as u64) as u32
                }
            }
        }
    };
}
impl_list_rate_limit!(Problem);
impl_list_rate_limit!(Contest);
impl_list_rate_limit!(Submit);
impl_list_rate_limit!(Education);
impl_list_rate_limit!(Testcase);
impl_list_rate_limit!(Announcement);
impl_list_rate_limit!(Token);

impl RateLimit for ListUserRequest {
    fn get_cost(&self) -> u32 {
        self.size
            .saturating_add(self.offset.unsigned_abs() / 6)
            .saturating_add(12)
            .min(u32::MAX as u64) as u32
    }
}

impl RateLimit for ListChatRequest {
    fn get_cost(&self) -> u32 {
        self.size
            .saturating_add(self.offset.unsigned_abs() / 8)
            .saturating_add(3)
            .min(u32::MAX as u64) as u32
    }
}

impl RateLimit for Id {}

macro_rules! impl_basic_rate_limit {
    ($t:ident) => {
        paste::paste! {
            impl RateLimit for [<Create $t Request>]{
                fn get_cost(&self) -> u32 {
                    17
                }
            }
            impl RateLimit for [<Update $t Request>]{
                fn get_cost(&self) -> u32 {
                    15
                }
            }
        }
    };
}
impl_basic_rate_limit!(Problem);
impl_basic_rate_limit!(Contest);
impl_basic_rate_limit!(Education);
impl_basic_rate_limit!(Testcase);
impl_basic_rate_limit!(Announcement);
impl_basic_rate_limit!(User);

impl RateLimit for UpdatePasswordRequest {
    fn get_cost(&self) -> u32 {
        230
    }
}
impl RateLimit for CreateSubmitRequest {
    fn get_cost(&self) -> u32 {
        430
    }
}
impl RateLimit for CreateChatRequest {
    fn get_cost(&self) -> u32 {
        10
    }
}

impl RateLimit for AddAnnouncementToContestRequest {}
impl RateLimit for AddEducationToProblemRequest {}
impl RateLimit for AddTestcaseToProblemRequest {}
impl RateLimit for AddProblemToContestRequest {}
impl RateLimit for JoinContestRequest {}
impl RateLimit for RejudgeRequest {}
impl RateLimit for LoginRequest {
    fn get_cost(&self) -> u32 {
        50
    }
}
impl RateLimit for () {}
impl RateLimit for RemoveRequest {}
impl RateLimit for PublishRequest {}
impl RateLimit for PublishContestRequest {}
impl RateLimit for ListAnnouncementByContestRequest {}
impl RateLimit for ListEducationByProblemRequest {}
impl RateLimit for UploadRequest {
    fn get_cost(&self) -> u32 {
        100
    }
}
impl RateLimit for ListTestcaseByProblemRequest {}
impl RateLimit for ListProblemByContestRequest {}
impl RateLimit for RefreshRequest {
    fn get_cost(&self) -> u32 {
        50
    }
}

impl RateLimit for InsertProblemRequest {
    fn get_cost(&self) -> u32 {
        3 + (self.pivot_id.is_some() as u32) * 2
    }
}
impl RateLimit for InsertTestcaseRequest {
    fn get_cost(&self) -> u32 {
        3 + (self.pivot_id.is_some() as u32) * 2
    }
}
