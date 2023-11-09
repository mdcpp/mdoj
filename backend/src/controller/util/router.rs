use std::{
    collections::{BTreeMap, VecDeque},
    ops::DerefMut,
    sync::{atomic::AtomicBool, Arc},
};

use lockfree::prelude::Map;
use spin::{Mutex, RwLock};
use tonic::transport;

use crate::{
    grpc::prelude::{judger_client::*, *},
    init::config,
};

use super::super::submit::Error;

const PIPELINE: usize = 8;

pub struct RouteRequest {
    pub match_rule: JudgeMatchRule,
    pub code: Vec<u8>,
    pub language: String,
}

// unavailable => available
// tls error => log + internal error
async fn connect_by_config(
    config: Arc<config::Judger>,
) -> Result<JudgerClient<transport::Channel>, Error> {
    todo!()
}

struct ConnGuard {
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
}

impl ConnPool {
    fn new(config: Arc<config::Judger>) -> Arc<Self> {
        Arc::new(Self {
            config,
            clients: Default::default(),
        })
    }
    async fn get(self: Arc<Self>) -> Result<ConnGuard, Error> {
        let pool = self.clone();
        Ok(match { self.clients.lock().pop_front() } {
            Some(client) => ConnGuard {
                pool,
                client: Some(client),
            },
            None => ConnGuard {
                pool,
                client: Some(connect_by_config(self.config.clone()).await?),
            },
        })
    }
}

// abstraction for langs info health check
// more frequent health check if ConnPool report unavailable
struct Upstream {
    config: Arc<config::Judger>,
    pool: ConnPool,
    langs: RwLock<BTreeMap<String, LangInfo>>,
    healthy: AtomicBool,
}

pub struct Router {
    // upstreams: Vec<Arc<Upstream>>,
    entry: Map<String, Arc<Upstream>>,
}

impl Router {
    pub async fn new(uri: transport::Uri) -> Result<Arc<Self>, Error> {
        // let client = JudgerClient::connect(uri).await?;
        todo!()
    }
}

// There are two type of router, single and multiple(hot reload)
// Router is responsible for
// 1. routing the request to the judger with corresponding language support
// 2. expose the language support to the endpoints
// 3. watch change of the running tasks, notify the endpoints with spsc channel
// 4. health check

// single router:
// very simple router, only one judger, no hot reloadable
// keep in mind don't overflow judger's buffer(256MiB)
// If it's going to overflow, return error to the endpoint

// multiple router:
// multiple judgers, hot reloadable, thick client
