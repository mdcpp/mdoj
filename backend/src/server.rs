use std::{ops::Deref, sync::Arc};

use crate::{
    controller::*,
    init::{
        config::{self, GlobalConfig},
        logger::{self, OtelGuard},
        Error,
    },
};
use grpc::backend::{
    announcement_server::AnnouncementServer, chat_server::ChatServer,
    contest_server::ContestServer, education_server::EducationServer,
    problem_server::ProblemServer, submit_server::SubmitServer, testcase_server::TestcaseServer,
    token_server::TokenServer, user_server::UserServer,
};
use http::header::HeaderName;
use sea_orm::DatabaseConnection;
use spin::Mutex;
use tonic::transport::{self, Identity, ServerTlsConfig};
use tonic_web::GrpcWebLayer;
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use tracing::{span, Instrument, Level};

const MAX_FRAME_SIZE: u32 = 1024 * 1024 * 8;

// from https://docs.rs/tonic-web/0.11.0/src/tonic_web/lib.rs.html#140
const DEFAULT_EXPOSED_HEADERS: [&str; 3] =
    ["grpc-status", "grpc-message", "grpc-status-details-bin"];
const DEFAULT_ALLOW_HEADERS: [&str; 5] = [
    "x-grpc-web",
    "content-type",
    "x-user-agent",
    "grpc-timeout",
    "token",
];

/// A wrapper to launch server
///
/// [`Server`] doesn't hold state
pub struct Server {
    pub token: Arc<token::TokenController>,
    pub judger: Arc<judger::Judger>,
    pub dup: duplicate::DupController,
    pub crypto: crypto::CryptoController,
    pub metrics: metrics::MetricsController,
    pub imgur: imgur::ImgurController,
    pub rate_limit: rate_limit::RateLimitController,
    pub config: GlobalConfig,
    pub db: Arc<DatabaseConnection>,
    pub identity: Mutex<Option<Identity>>,
    _otel_guard: OtelGuard,
}

#[derive(Clone)]
pub struct ArcServer(Arc<Server>);

impl Deref for ArcServer {
    type Target = Server;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Server {
    /// Create a new server
    ///
    /// It will initialize project's stateful components in following order:
    /// 1. Config
    /// 2. Logger
    /// 3. Crypto Controller
    /// 4. Other Controller
    ///
    /// Also of note, private/public `*.pem` is loaded during [`Server::start`] instead of this function
    pub async fn new() -> Result<Arc<Self>, Error> {
        let config = config::init().await?;

        let otel_guard = logger::init(&config)?;
        let span = span!(Level::INFO, "server_construct");
        let crypto = crypto::CryptoController::new(&config, &span);
        let db = Arc::new(
            crate::init::db::init(&config.database, &crypto, &span)
                .in_current_span()
                .await?,
        );

        let mut identity = None;
        if config.grpc.private_pem.is_some() {
            let private_pem = config.grpc.private_pem.as_ref().unwrap();
            let public_pem = config
                .grpc
                .public_pem
                .as_ref()
                .expect("public pem should set if private pem is set");

            let cert = std::fs::read_to_string(public_pem).map_err(Error::ReadPem)?;
            let key = std::fs::read_to_string(private_pem).map_err(Error::ReadPem)?;

            identity = Some(Identity::from_pem(cert, key));
        }

        Ok(Arc::new(Server {
            token: token::TokenController::new(&span, db.clone()),
            judger: Arc::new(
                judger::Judger::new(config.judger.clone(), db.clone(), &span)
                    .in_current_span()
                    .await
                    .unwrap(),
            ),
            dup: duplicate::DupController::new(&span),
            crypto,
            metrics: metrics::MetricsController::new(&otel_guard.meter_provider),
            imgur: imgur::ImgurController::new(&config.imgur),
            rate_limit: rate_limit::RateLimitController::new(&config.grpc.trust_host),
            config,
            identity: Mutex::new(identity),
            db,
            _otel_guard: otel_guard,
        }))
    }
    /// Start the server
    pub async fn start(self: Arc<Self>) {
        let self_ = ArcServer(self);
        let cors = CorsLayer::new()
            .allow_headers(
                DEFAULT_ALLOW_HEADERS
                    .iter()
                    .cloned()
                    .map(HeaderName::from_static)
                    .collect::<Vec<HeaderName>>(),
            )
            .expose_headers(
                DEFAULT_EXPOSED_HEADERS
                    .iter()
                    .cloned()
                    .map(HeaderName::from_static)
                    .collect::<Vec<HeaderName>>(),
            )
            .allow_origin(AllowOrigin::mirror_request())
            .allow_methods(Any);

        let server = match self_.0.identity.lock().take() {
            Some(identity) => transport::Server::builder()
                .tls_config(ServerTlsConfig::new().identity(identity))
                .unwrap(),
            None => transport::Server::builder().accept_http1(true),
        };

        server
            .layer(cors)
            .layer(GrpcWebLayer::new())
            .max_frame_size(Some(MAX_FRAME_SIZE))
            .add_service(ProblemServer::new(self_.clone()))
            .add_service(EducationServer::new(self_.clone()))
            .add_service(UserServer::new(self_.clone()))
            .add_service(TokenServer::new(self_.clone()))
            .add_service(ContestServer::new(self_.clone()))
            .add_service(TestcaseServer::new(self_.clone()))
            .add_service(SubmitServer::new(self_.clone()))
            .add_service(ChatServer::new(self_.clone()))
            .add_service(AnnouncementServer::new(self_.clone()))
            .serve_with_shutdown(
                self_.0.config.bind_address.clone().parse().unwrap(),
                async {
                    if tokio::signal::ctrl_c().await.is_err() {
                        tracing::warn!("graceful_shutdown");
                    }
                },
            )
            .await
            .unwrap();
    }
}
