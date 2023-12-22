pub mod direct;
pub mod swarm;

use super::Error;
use std::{
    ops::DerefMut,
    sync::{
        atomic::{AtomicIsize, Ordering},
        Arc, Weak,
    },
    time::Duration,
};

use crossbeam_queue::SegQueue;
use dashmap::{DashMap, DashSet};
use tonic::{service::Interceptor, *};
use tracing::{instrument, span, Instrument, Level, Span};
use uuid::Uuid;

use crate::{
    grpc::judger::{judger_client::*, *},
    init::config::{self, Judger as JudgerConfig},
};

// introduce routing layer error
// type Map<K, V> = Mutex<HashMap<K, V>>;
// type Queue<V> = Mutex<VecDeque<V>>;
// type Set<V> = Mutex<HashSet<V>>;
const HEALTHY_THRESHOLD: isize = 100;
type JudgerIntercept = JudgerClient<
    service::interceptor::InterceptedService<transport::Channel, BasicAuthInterceptor>,
>;

pub struct BasicAuthInterceptor {
    secret: Option<String>,
}

impl Interceptor for BasicAuthInterceptor {
    fn call(&mut self, mut req: tonic::Request<()>) -> Result<tonic::Request<()>, Status> {
        match &self.secret {
            Some(secret) => {
                let token = secret.parse().unwrap();
                req.metadata_mut().insert("Authorization", token);
                Ok(req)
            }
            None => Ok(req),
        }
    }
}

#[derive(Clone)]
pub struct ConnectionDetail {
    pub uri: String,
    pub secret: Option<String>,
    pub reuse: bool,
}

impl ConnectionDetail {
    async fn connect(&self) -> Result<JudgerIntercept, Error> {
        let channel = transport::Channel::from_shared(self.uri.clone())
            .unwrap()
            .connect()
            .await?;

        let interceptor = BasicAuthInterceptor {
            secret: self.secret.as_ref().map(|x| ["basic ", x].concat()),
        };

        Ok(JudgerClient::with_interceptor(channel, interceptor))
    }
}

pub struct ConnGuard {
    upstream: Arc<Upstream>,
    conn: Option<JudgerIntercept>,
    reuse: bool,
}

impl ConnGuard {
    pub fn report_success(&mut self) {
        self.upstream.healthy.fetch_add(3, Ordering::Acquire);
        self.upstream
            .healthy
            .fetch_min(HEALTHY_THRESHOLD, Ordering::Acquire);
    }
}

impl DerefMut for ConnGuard {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.conn.as_mut().unwrap()
    }
}

impl std::ops::Deref for ConnGuard {
    type Target = JudgerIntercept;
    fn deref(&self) -> &Self::Target {
        self.conn.as_ref().unwrap()
    }
}

impl Drop for ConnGuard {
    fn drop(&mut self) {
        self.upstream.healthy.fetch_add(-2, Ordering::Acquire);
        if self.reuse{
            self.upstream.clients.push(self.conn.take().unwrap());
        }
    }
}

async fn discover<I: Routable + Send>(
    config: JudgerConfig,
    router: Weak<Router>,
) -> Result<(), Error> {
    let mut instance = I::new(config.clone())?;
    let span = span!(Level::INFO, "service_discover", config_name = config.name);
    loop {
        match instance
            .discover()
            .instrument(span!(parent:span.clone(),Level::DEBUG, "try advance"))
            .in_current_span()
            .await
        {
            RouteStatus::NewConnection(detail) => {
                let _span =
                    span!(parent:span.clone(),Level::DEBUG,"upstream_connect",uri=detail.uri);
                let router = match router.upgrade() {
                    Some(x) => x,
                    None => break,
                };
                let (upstream, langs) = Upstream::new(detail).in_current_span().await?;
                for (uuid, lang) in langs.into_iter() {
                    let _ = tracing::span!(parent:&_span,Level::DEBUG,"lang_insert",uuid=?&uuid)
                        .entered();
                    router.langs.insert(lang);
                    loop {
                        match router.routing_table.get(&uuid) {
                            Some(x) => {
                                x.push(upstream.clone());
                                break;
                            }
                            None => {
                                router.routing_table.insert(uuid, Default::default());
                            }
                        }
                    }
                }
            }
            RouteStatus::Wait(dur) => tokio::time::sleep(dur).in_current_span().await,
            _ => break,
        }
    }
    Ok(())
}

pub struct Router {
    routing_table: DashMap<Uuid, SegQueue<Arc<Upstream>>>,
    pub langs: DashSet<LangInfo>,
}

impl Router {
    // skip because config contain basic auth secret
    #[instrument(name="router_construct",level = "info",skip_all, follows_from = [span])]
    pub async fn new(config: Vec<JudgerConfig>, span: &Span) -> Result<Arc<Self>, Error> {
        let self_ = Arc::new(Self {
            routing_table: DashMap::default(),
            langs: DashSet::default(),
        });
        for config in config.into_iter() {
            match config.judger_type {
                config::JudgerType::Docker => {
                    tokio::spawn(discover::<swarm::DockerRouter>(
                        config,
                        Arc::downgrade(&self_),
                    ));
                }
                config::JudgerType::Static => {
                    tokio::spawn(discover::<direct::StaticRouter<true>>(
                        config,
                        Arc::downgrade(&self_),
                    ));
                }
                config::JudgerType::LoadBalanced => {
                    tokio::spawn(discover::<direct::StaticRouter<false>>(
                        config,
                        Arc::downgrade(&self_),
                    ));
                }
            }
        }
        Ok(self_)
    }
    pub async fn get(&self, lang: &Uuid) -> Result<ConnGuard, Error> {
        let queue = self
            .routing_table
            .get(lang)
            .ok_or(Error::BadArgument("lang"))?;

        loop {
            match queue.pop() {
                Some(upstream) => {
                    if upstream.is_healthy() {
                        queue.push(upstream.clone());
                        return upstream.get().await;
                    }
                }
                None => {
                    self.routing_table.remove(lang);
                    return Err(Error::BadArgument("lang"));
                }
            }
        }
    }
}

// abstraction for pipelining
pub struct Upstream {
    healthy: AtomicIsize,
    clients: SegQueue<JudgerIntercept>,
    connection: ConnectionDetail,
    // live_span: tracing::span::EnteredSpan,
}

impl Upstream {
    async fn new(detail: ConnectionDetail) -> Result<(Arc<Self>, Vec<(Uuid, LangInfo)>), Error> {
        let mut client = detail.connect().await?;
        let info = client.judger_info(()).await?;
        let langs = info.into_inner().langs.list;

        let mut result = Vec::new();
        for lang in langs.into_iter() {
            let uuid = match Uuid::parse_str(&lang.lang_uid) {
                Ok(x) => x,
                Err(err) => {
                    log::warn!("invalid lang_uid from judger: {}", err);
                    continue;
                }
            };
            result.push((uuid, lang));
        }

        let clients = SegQueue::default();
        clients.push(client);

        Ok((
            Arc::new(Self {
                healthy: AtomicIsize::new(HEALTHY_THRESHOLD),
                clients,
                connection: detail,
            }),
            result,
        ))
    }
    fn is_healthy(&self) -> bool {
        self.healthy.load(Ordering::Acquire) > 0
    }
    async fn get(self: Arc<Self>) -> Result<ConnGuard, Error> {
        let conn = match self.clients.pop() {
            Some(x) => x,
            None => self.connection.connect().await?,
        };

        Ok(ConnGuard {
            reuse: self.connection.reuse,
            upstream: self,
            conn: Some(conn),
        })
    }
}

pub enum RouteStatus {
    NewConnection(ConnectionDetail),
    Wait(Duration),
    Never,
    Abort,
}

#[async_trait]
pub trait Routable
where
    Self: Sized,
{
    // return new connection when available, will immediately retry true is returned
    async fn route(&mut self) -> Result<RouteStatus, Error>;
    fn new(config: JudgerConfig) -> Result<Self, Error>;
}

#[async_trait]
pub trait Discoverable {
    // return new connection when available, will immediately retry true is returned
    async fn discover(&mut self) -> RouteStatus;
}

// trait Constructable
// where
//     Self: Sized,
// {
//     fn new(config: JudgerConfig) -> Result<Self, Error>;
// }

#[async_trait]
impl<S: Routable + Send> Discoverable for S {
    async fn discover(&mut self) -> RouteStatus {
        match self.route().await {
            Ok(x) => x,
            Err(err) => {
                log::warn!("{}", err);
                RouteStatus::Abort
            }
        }
    }
}

// #[async_trait]
// impl<S: Routable + Send> Constructable for S {
//     fn new(config: JudgerConfig) -> Result<Self, Error> {
//         Self::new(config)
//     }
// }
