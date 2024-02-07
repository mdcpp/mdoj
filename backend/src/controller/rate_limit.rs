use std::{hash::Hash, net::IpAddr, num::NonZeroU32, str::FromStr, sync::Arc};

use crate::NonZeroU32;
use futures::Future;
use governor::{DefaultDirectRateLimiter, DefaultKeyedRateLimiter, Quota, RateLimiter};
use ip_network::IpNetwork;
use quick_cache::sync::Cache;

use tracing::{instrument, Instrument};

use crate::util::error::Error;

const BUCKET_WIDTH: usize = 512;

/// Policy(number) for rate limit
trait LimitPolicy {
    const BUCKET_WIDTH: usize = BUCKET_WIDTH;
    /// How many burst request is allowed
    const BURST: NonZeroU32;
    /// How many fill per minute
    const RATE: NonZeroU32;

    fn direct() -> DefaultDirectRateLimiter {
        RateLimiter::direct(Quota::per_minute(Self::RATE).allow_burst(Self::BURST))
    }
    fn key<K>() -> DefaultKeyedRateLimiter<K>
    where
        K: Send + Eq + Hash + Clone,
    {
        RateLimiter::keyed(Quota::per_minute(Self::RATE).allow_burst(Self::BURST))
    }
}

/// policy for [`TrafficType::Login`]
struct LoginPolicy;

impl LimitPolicy for LoginPolicy {
    const BURST: NonZeroU32 = NonZeroU32!(200);
    const RATE: NonZeroU32 = NonZeroU32!(55);
}

/// policy for [`TrafficType::Guest`]
struct GuestPolicy;

impl LimitPolicy for GuestPolicy {
    const BURST: NonZeroU32 = NonZeroU32!(80);
    const RATE: NonZeroU32 = NonZeroU32!(35);
}

/// policy for [`TrafficType::Blacklist`]
///
/// note that this is a global rate limit,
/// users in blacklist use same [`leaky_bucket::RateLimiter`],
/// so number is significantly higher
struct BlacklistPolicy;

impl LimitPolicy for BlacklistPolicy {
    const BURST: NonZeroU32 = NonZeroU32!(30);
    const RATE: NonZeroU32 = NonZeroU32!(10);
}

pub struct RateLimitController {
    ip_blacklist: Cache<IpAddr, ()>,
    user_limiter: Arc<DefaultKeyedRateLimiter<i32>>,
    ip_limiter: Arc<DefaultKeyedRateLimiter<IpAddr>>,
    blacklist_limiter: Arc<DefaultDirectRateLimiter>,
    trusts: Vec<IpNetwork>,
}

/// Type of traffic
pub enum TrafficType {
    /// Login user(with vaild token)
    Login(i32),
    /// Guest(without token)
    Guest,
    /// traffic with token from blacklisted ip
    ///
    /// see [`RateLimitController::check`]
    Blacklist(crate::controller::token::Error),
}

impl RateLimitController {
    pub fn new(trusts: &[IpNetwork]) -> Self {
        Self {
            ip_blacklist: Cache::new(BUCKET_WIDTH),
            user_limiter: Arc::new(LoginPolicy::key()),
            ip_limiter: Arc::new(LoginPolicy::key()),
            blacklist_limiter: Arc::new(BlacklistPolicy::direct()),
            trusts: trusts.to_vec(),
        }
    }
    /// retrieve ip address from request
    ///
    /// if used on unix socket return 0.0.0.0
    ///
    /// if upstream is trusted but sent no `X-Forwarded-For`, use remote address
    #[instrument(skip_all, level = "trace")]
    fn ip<T>(&self, req: &tonic::Request<T>) -> Result<IpAddr, Error> {
        let mut remote = req
            .remote_addr()
            .map(|x| x.ip())
            .unwrap_or_else(|| IpAddr::from_str("0.0.0.0").unwrap());

        tracing::trace!(remote = remote.to_string());

        for trust in &self.trusts {
            if !trust.contains(remote) {
                if let Some(addr) = req.metadata().get("X-Forwarded-For") {
                    remote = addr
                        .to_str()
                        .map_err(|_| Error::Unreachable("header must not contain non-ascii char"))?
                        .parse()
                        .map_err(|_| Error::Unreachable("MalFormatted header"))?;
                }
            }
        }

        Ok(remote)
    }
    /// check rate limit
    ///
    /// f should be a FnOnce that emit a future yield TokenState
    ///
    /// There are three type of traffic
    ///
    /// - [`TrafficType::Login`]: faster rate and apply rate limit base on user id
    /// - [`TrafficType::Guest`]: slower rate and apply rate limit base on ip address
    /// - [`TrafficType::Blacklist`]: dedicated rate limit (because verify token take time)
    ///
    /// We identify [`TrafficType::Blacklist`] by ip blacklist,
    /// whose entries is added when user fail to login or sent invaild token
    #[instrument(skip_all, level = "debug")]
    pub async fn check<'a, T, F, Fut>(
        &self,
        req: &'a tonic::Request<T>,
        permit: NonZeroU32,
        f: F,
    ) -> Result<(), Error>
    where
        F: FnOnce(&'a tonic::Request<T>) -> Fut,
        Fut: Future<Output = TrafficType>,
    {
        // transform bool to Result<(),Error>
        macro_rules! bool_err {
            ($e:expr) => {
                match $e {
                    true => Ok(()),
                    false => Err(Error::RateLimit),
                }
            };
        }
        let addr = self.ip(req)?;

        if self.ip_blacklist.get(&addr).is_some() {
            return bool_err!(self.blacklist_limiter.check_n(permit).is_ok());
        }

        let is_limited = match f(req)
            .instrument(tracing::debug_span!("token_verify"))
            .await
        {
            TrafficType::Login(x) => self.user_limiter.check_key_n(&x, permit),
            TrafficType::Guest => self.ip_limiter.check_key_n(&addr, permit),
            TrafficType::Blacklist(err) => {
                tracing::warn!(msg = err.to_string(), "ip_blacklist");
                self.ip_blacklist.insert(addr, ());
                self.blacklist_limiter.check_n(permit)
            }
        };

        bool_err!(is_limited.is_ok())
    }
}
