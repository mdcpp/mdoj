use std::{hash::Hash, net::IpAddr, num::NonZeroU32, str::FromStr, sync::Arc};

use crate::NonZeroU32;
use futures::Future;
use governor::{DefaultDirectRateLimiter, DefaultKeyedRateLimiter, Quota, RateLimiter};
use ip_network::IpNetwork;
use quick_cache::sync::Cache;

use tracing::{instrument, Instrument};

use crate::util::error::Error;

const BUCKET_WIDTH: usize = 512;

/// A bucket that can have limit applied
///
/// The reason we don't provide earliest possible time
/// is that it require access to quanta to compute duration
///
/// We implement keyed bucket and unkey bucket with enum
/// for faster allocation
pub enum Bucket {
    Guest((Arc<DefaultKeyedRateLimiter<IpAddr>>, IpAddr)),
    Login((Arc<DefaultKeyedRateLimiter<i32>>, i32)),
    Blacklist(Arc<DefaultDirectRateLimiter>),
}

impl Bucket {
    fn expect_dur(&self, cost: NonZeroU32) -> bool {
        let res = match self {
            Bucket::Guest((limiter, key)) => limiter.check_key_n(&key, cost),
            Bucket::Login((limiter, key)) => limiter.check_key_n(&key, cost),
            Bucket::Blacklist(limiter) => limiter.check_n(cost),
        };
        match res {
            Ok(res) => res.is_err(),
            Err(_) => true,
        }
    }
    pub fn cost(&self, cost: NonZeroU32) -> Result<(), Error> {
        match self.expect_dur(cost) {
            true => Err(Error::RateLimit("")),
            false => Ok(()),
        }
    }
}

/// Policy(accounting on endpoint) for rate limit
trait EndpointPolicy {
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

impl EndpointPolicy for LoginPolicy {
    const BURST: NonZeroU32 = NonZeroU32!(400);
    const RATE: NonZeroU32 = NonZeroU32!(150);
}

/// policy for [`TrafficType::Guest`]
struct GuestPolicy;

impl EndpointPolicy for GuestPolicy {
    const BURST: NonZeroU32 = NonZeroU32!(150);
    const RATE: NonZeroU32 = NonZeroU32!(80);
}

/// policy for [`TrafficType::Blacklist`]
///
/// note that this is a global rate limit,
/// users in blacklist use same [`leaky_bucket::RateLimiter`],
/// so number is significantly higher
struct BlacklistPolicy;

impl EndpointPolicy for BlacklistPolicy {
    const BURST: NonZeroU32 = NonZeroU32!(60);
    const RATE: NonZeroU32 = NonZeroU32!(30);
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
        f: F,
    ) -> Result<Bucket, Error>
    where
        F: FnOnce(&'a tonic::Request<T>) -> Fut,
        Fut: Future<Output = TrafficType>,
    {
        let addr = self.ip(req)?;

        if self.ip_blacklist.get(&addr).is_some() {
            return Ok(Bucket::Blacklist(self.blacklist_limiter.clone()));
        }

        let res = match f(req)
            .instrument(tracing::debug_span!("token_verify"))
            .await
        {
            TrafficType::Login(x) => Bucket::Login((self.user_limiter.clone(), x)),
            TrafficType::Guest => Bucket::Guest((self.ip_limiter.clone(), addr)),
            TrafficType::Blacklist(err) => {
                tracing::warn!(msg = err.to_string(), "ip_blacklist");
                self.ip_blacklist.insert(addr, ());
                Bucket::Blacklist(self.blacklist_limiter.clone())
            }
        };

        Ok(res)
    }
}
