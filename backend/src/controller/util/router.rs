use std::{
    collections::{BTreeMap, VecDeque},
    ops::DerefMut,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
};

use spin::{Mutex, RwLock};
use tonic::transport;

use crate::{
    grpc::judger::{judger_client::*, judger_server::Judger, *},
    init::config,
};

use super::super::submit::Error;

const PIPELINE: usize = 8;
const JUDGER_QUE_MAX: usize = 16;
const HEALTHCHECK_DURATION: std::time::Duration = std::time::Duration::from_secs(60);

pub struct RouteRequest {
    pub match_rule: JudgeMatchRule,
    pub code: Vec<u8>,
    pub language: String,
}

// unavailable => available
// tls error => log + internal error
async fn connect_by_config(
    config: &config::Judger,
) -> Result<JudgerClient<transport::Channel>, Error> {
    JudgerClient::connect(config.uri.clone())
        .await
        .map_err(|err| {
            log::warn!("judger {} is unavailable: {}", config.uri, err);
            Error::JudgerUnavailable
        })
}

pub struct ConnGuard {
    pool: Arc<ConnPool>,
    client: Option<JudgerClient<transport::Channel>>,
}

impl Drop for ConnGuard {
    fn drop(&mut self) {
        let mut lock = self.pool.clients.lock();
        if lock.len() < PIPELINE {
            if let Some(x) = self.client.take() {
                lock.push_back(x);
            }
        }
    }
}
impl std::ops::Deref for ConnGuard {
    type Target = JudgerClient<transport::Channel>;
    fn deref(&self) -> &Self::Target {
        self.client.as_ref().unwrap()
    }
}
impl DerefMut for ConnGuard {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.client.as_mut().unwrap()
    }
}

// Abstraction for pipelining, reconnect logic
struct ConnPool {
    config: Arc<config::Judger>,
    clients: Mutex<VecDeque<JudgerClient<transport::Channel>>>,
    running: AtomicUsize,
    healthy: AtomicBool,
}

impl ConnPool {
    fn new(config: Arc<config::Judger>) -> Arc<Self> {
        Arc::new(Self {
            config,
            clients: Default::default(),
            running: Default::default(),
            healthy: Default::default(),
        })
    }
    async fn get(self: &Arc<Self>, max: usize) -> Result<ConnGuard, Error> {
        if self.running.fetch_add(1, Ordering::Relaxed) > max {
            self.running.fetch_sub(1, Ordering::Relaxed);
            self.healthy.store(false, Ordering::Relaxed);
            return Err(Error::JudgerUnavailable);
        };
        let pool = self.clone();
        Ok(match { self.clients.lock().pop_front() } {
            Some(client) => ConnGuard {
                pool,
                client: Some(client),
            },
            None => ConnGuard {
                pool,
                client: Some(connect_by_config(&self.config).await?),
            },
        })
    }
}

// abstraction for langs info health check
// more frequent health check if ConnPool report unavailable
struct Upstream {
    config: Arc<config::Judger>,
    pool: Arc<ConnPool>,
    langs: RwLock<BTreeMap<String, LangInfo>>,
    accuracy: RwLock<u64>,
}

impl Upstream {
    fn new(config: Arc<config::Judger>) -> Arc<Upstream> {
        let self_ = Arc::new(Self {
            config: config.clone(),
            pool: ConnPool::new(config),
            langs: Default::default(),
            accuracy: Default::default(),
        });

        let self_weak = Arc::downgrade(&self_);
        tokio::spawn(async move {
            log::debug!("judger health check started");
            while let Some(self_) = self_weak.upgrade() {
                tokio::time::sleep(HEALTHCHECK_DURATION).await;
                self_.health_check().await.ok();
            }
            log::trace!("judger health check exited");
        });

        self_
    }
    async fn health_check(&self) -> Result<(), Error> {
        macro_rules! health {
            ($e:expr) => {
                $e.await.map_err(|x| {
                    log::warn!("judger health check failed: {}", x);
                    Error::HealthCheck
                })?
            };
        }
        let mut conn = health!(self.pool.get(usize::MAX));
        let info = health!(conn.judger_info(()));

        let res = info.into_inner();

        let langs: BTreeMap<String, LangInfo> = match res.langs {
            Some(x) => x.list,
            None => Vec::new(),
        }
        .into_iter()
        .map(|x| (x.lang_uid.clone(), x))
        .collect();

        *self.accuracy.write() = res.accuracy;
        *self.langs.write() = langs;

        Ok(())
    }
}

pub struct Router {
    upstreams: Vec<Arc<Upstream>>,
    next_entry: AtomicUsize,
}

impl Router {
    pub async fn new(configs: &[Arc<config::Judger>]) -> Result<Arc<Router>, Error> {
        let mut upstreams = Vec::new();
        for config in configs {
            let upstream = Upstream::new(config.clone());
            if let Err(err) = upstream.health_check().await {
                log::warn!("judger {} is unavailable: {}", config.uri, err);
                continue;
            }
            upstreams.push(upstream);
        }
        // let futs: Box<dyn Future<Output = Arc<Upstream>>> = configs
        //     .iter()
        //     .map(|x| async {
        //         let upstream = Upstream::new(Arc::new(x.clone()));
        //         upstream.health_check().await;
        //         upstream
        //     })
        //     .collect();
        // tokio::join!(futs);
        if upstreams.is_empty() {
            return Err(Error::JudgerUnavailable);
        }
        Ok(Arc::new(Self {
            upstreams,
            next_entry: AtomicUsize::new(0),
        }))
    }
    pub async fn get(&self, uid: &String) -> Result<ConnGuard, Error> {
        let server_count = self.upstreams.len();
        for _ in 0..(server_count * 2 + 1) {
            let next = self.next_entry.fetch_add(1, Ordering::Relaxed) % server_count;
            let upstream = &self.upstreams[next];
            if upstream.pool.healthy.load(Ordering::Relaxed)
                && upstream.langs.read().contains_key(uid)
            {
                match upstream.pool.get(JUDGER_QUE_MAX).await {
                    Ok(x) => return Ok(x),
                    Err(err) => {
                        log::warn!("judger {} is unavailable: {}", upstream.config.uri, err);
                    }
                }
            }
        }
        log::warn!("no judger available");
        Err(Error::JudgerUnavailable)
    }
}
