use core::time;
use std::{net::IpAddr, sync::Arc};

use ip_network::IpNetwork;
use leaky_bucket::RateLimiter;
use quick_cache::sync::Cache;

use tracing::instrument;

use crate::util::error::Error;

const BUCKET_WIDTH: usize = 256;

pub struct RateLimitController {
    limiter: Cache<IpAddr, Arc<RateLimiter>>,
    trusts: Vec<IpNetwork>,
}

macro_rules! check_rate_limit {
    ($s:expr) => {{
        use futures::Future;
        struct Waker;
        impl std::task::Wake for Waker {
            fn wake(self: Arc<Self>) {
                log::error!("waker wake");
            }
        }
        let waker = Arc::new(Waker).into();
        let mut cx = std::task::Context::from_waker(&waker);

        let ac = $s;
        tokio::pin!(ac);
        if ac.as_mut().poll(&mut cx).is_pending() {
            return Err(Error::RateLimit);
        }
    }};
}

impl RateLimitController {
    pub fn new(trusts: &[IpNetwork]) -> Self {
        Self {
            limiter: Cache::new(BUCKET_WIDTH),
            trusts: trusts.to_vec(),
        }
    }
    #[instrument(skip_all, level = "debug")]
    pub fn check_ip<T>(&self, req: &tonic::Request<T>, permits: usize) -> Result<(), Error> {
        if self.trusts.is_empty() {
            return Ok(());
        }
        if req.remote_addr().is_none() {
            tracing::warn!(msg = "cannot not retrieve remote address", "config");
            return Ok(());
        }
        let remote = req.remote_addr().unwrap().ip();
        for trust in &self.trusts {
            if !trust.contains(remote) {
                continue;
            }
            if let Some(ip) = req.metadata().get("X-Forwarded-For") {
                let ip = ip
                    .to_str()
                    .map_err(|_| Error::Unreachable("header must not contain non-ascii char"))?
                    .parse()
                    .map_err(|_| Error::Unreachable("MalFormatted header"))?;
                return self.acquire(ip, permits);
            } else {
                tracing::warn!(msg = "No \"X-Forwarded-For\" found", "config");
            }
        }
        Err(Error::RateLimit)
    }

    #[instrument(skip_all, level = "debug")]
    fn acquire(&self, ip: IpAddr, permits: usize) -> Result<(), Error> {
        let limiter = self
            .limiter
            .get_or_insert_with::<_, ()>(&ip, || {
                Ok(Arc::new(
                    RateLimiter::builder()
                        .max(40)
                        .initial(10)
                        .interval(time::Duration::from_secs(3))
                        .build(),
                ))
            })
            .map_err(|_| Error::Unreachable("creation function for limiter shouldn't panic"))?;
        let owned = limiter.acquire_owned(permits);

        check_rate_limit!(owned);
        Ok(())
    }
}

// impl Default for RateLimitController {
//     fn default() -> Self {
//         Self {
//             limiter: Cache::new(256),
//         }
//     }
// }
