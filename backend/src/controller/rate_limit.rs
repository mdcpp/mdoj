use std::{hash::Hash, marker::PhantomData, net::IpAddr, str::FromStr, sync::Arc, time::Duration};

use futures::Future;
use ip_network::IpNetwork;
use leaky_bucket::RateLimiter;
use quick_cache::sync::Cache;

use tracing::{instrument, Instrument};

use crate::util::error::Error;

const BUCKET_WIDTH: usize = 512;

/// Policy(number) for rate limit
trait LimitPolicy {
    const BUCKET_WIDTH: usize = BUCKET_WIDTH;
    const INIT_CAP: usize;
    const MAX_CAP: usize;
    /// How many fill per 30 second
    const FILL_RATE: usize;

    fn into_limiter() -> RateLimiter {
        RateLimiter::builder()
            .interval(Duration::from_secs(30))
            .initial(Self::INIT_CAP)
            .max(Self::MAX_CAP)
            .refill(Self::FILL_RATE)
            .build()
    }
}

/// policy for [`TrafficType::Login`]
struct LoginPolicy;

impl LimitPolicy for LoginPolicy {
    const INIT_CAP: usize = 32;
    const MAX_CAP: usize = 64;
    const FILL_RATE: usize = 6;
}

/// policy for [`TrafficType::Guest`]
struct GuestPolicy;

impl LimitPolicy for GuestPolicy {
    const INIT_CAP: usize = 32;
    const MAX_CAP: usize = 64;
    const FILL_RATE: usize = 6;
}

/// policy for [`TrafficType::Blacklist`]
///
/// note that this is a global rate limit,
/// users in blacklist use same [`leaky_bucket::RateLimiter`],
/// so number is significantly higher
struct BlacklistPolicy;

impl LimitPolicy for BlacklistPolicy {
    const INIT_CAP: usize = 128;
    const MAX_CAP: usize = 256;
    const FILL_RATE: usize = 32;
}

struct LimitMap<K, P: LimitPolicy>
where
    K: Send + Eq + Hash + Clone,
{
    cache: Cache<K, Arc<RateLimiter>>,
    _policy: PhantomData<P>,
}

/// interface that it's able to calculate rate limit ans store state (by key)
trait Limit<K: Send> {
    /// return true if limited
    fn check(&self, key: &K) -> bool;
    /// return `Err(Error::RateLimit)` when limitation reached,
    /// `Ok(())` otherwise.
    fn check_error(&self, key: &K) -> Result<(), Error> {
        match self.check(key) {
            true => Err(Error::RateLimit),
            false => Ok(()),
        }
    }
}

impl Limit<()> for Arc<RateLimiter> {
    fn check(&self, _: &()) -> bool {
        struct Waker;
        impl std::task::Wake for Waker {
            fn wake(self: Arc<Self>) {
                unreachable!("waker wake");
            }
        }

        let waker = Arc::new(Waker).into();
        let mut cx = std::task::Context::from_waker(&waker);

        let ac = self.clone().acquire_owned(1);
        tokio::pin!(ac);

        ac.as_mut().poll(&mut cx).is_pending()
    }
}

impl<K, P: LimitPolicy> Limit<K> for LimitMap<K, P>
where
    K: Send + Eq + Hash + Clone,
{
    fn check(&self, key: &K) -> bool {
        self.cache
            .get_or_insert_with(key, || Result::<_, ()>::Ok(Arc::new(P::into_limiter())))
            .unwrap()
            .check(&())
    }
}

impl<K, P: LimitPolicy> Default for LimitMap<K, P>
where
    K: Send + Eq + Hash + Clone,
{
    fn default() -> Self {
        Self {
            cache: Cache::new(P::BUCKET_WIDTH),
            _policy: Default::default(),
        }
    }
}

pub struct RateLimitController {
    ip_blacklist: Cache<IpAddr, ()>,
    user_limiter: LimitMap<i32, LoginPolicy>,
    ip_limiter: LimitMap<IpAddr, GuestPolicy>,
    blacklist_limiter: Arc<RateLimiter>,
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
            user_limiter: Default::default(),
            ip_limiter: Default::default(),
            blacklist_limiter: Arc::new(BlacklistPolicy::into_limiter()),
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
    pub async fn check<'a, T, F, Fut>(&self, req: &'a tonic::Request<T>, f: F) -> Result<(), Error>
    where
        F: FnOnce(&'a tonic::Request<T>) -> Fut,
        Fut: Future<Output = TrafficType>,
    {
        let addr = self.ip(req)?;

        if self.ip_blacklist.get(&addr).is_some() {
            return self.ip_limiter.check_error(&addr);
        }

        match f(req)
            .instrument(tracing::debug_span!("token_verify"))
            .await
        {
            TrafficType::Login(x) => self.user_limiter.check_error(&x),
            TrafficType::Guest => self.ip_limiter.check_error(&addr),
            TrafficType::Blacklist(err) => {
                tracing::warn!(msg = err.to_string(), "ip_blacklist");
                self.ip_blacklist.insert(addr, ());
                self.blacklist_limiter.check_error(&())
            }
        }
    }
}
