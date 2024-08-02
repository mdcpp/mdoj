pub mod db;
pub mod error;
pub mod logger;

pub use error::InitError;
pub type Result<T> = std::result::Result<T, InitError>;

pub use logger::OtelGuard;

use std::{ops::Deref, sync::Arc};

use crate::controller::*;

use crate::config::CONFIG;
use grpc::backend::{
    announcement_server::AnnouncementServer, chat_server::ChatServer,
    contest_server::ContestServer, education_server::EducationServer,
    problem_server::ProblemServer, submit_server::SubmitServer, testcase_server::TestcaseServer,
    token_server::TokenServer, user_server::UserServer,
};
use http::header::HeaderName;
use opentelemetry::trace::FutureExt;
use sea_orm::DatabaseConnection;
use spin::Mutex;
use tonic::transport::{self, Identity, ServerTlsConfig};
use tonic_web::GrpcWebLayer;
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use tracing::Instrument;

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
    pub crypto: crypto::CryptoController,
    pub imgur: imgur::ImgurController,
    pub rate_limit: rate_limit::RateLimitController,
    pub db: Arc<DatabaseConnection>,
    pub identity: Mutex<Option<Identity>>,
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
    pub async fn new() -> Result<Arc<Self>> {
        let crypto = crypto::CryptoController::new();
        let db = Arc::new(
            db::init(&CONFIG.database, &crypto)
                .with_current_context()
                .await?,
        );

        let mut identity = None;
        if CONFIG.grpc.private_pem.is_some() {
            let private_pem = CONFIG.grpc.private_pem.as_ref().unwrap();
            let public_pem = CONFIG
                .grpc
                .public_pem
                .as_ref()
                .expect("public pem should set if private pem is set");

            let cert = std::fs::read_to_string(public_pem).map_err(InitError::ReadPem)?;
            let key = std::fs::read_to_string(private_pem).map_err(InitError::ReadPem)?;

            identity = Some(Identity::from_pem(cert, key));
        }

        Ok(Arc::new(Server {
            token: token::TokenController::new(db.clone()),
            judger: Arc::new(
                judger::Judger::new(db.clone())
                    .in_current_span()
                    .await
                    .unwrap(),
            ),
            crypto,
            imgur: imgur::ImgurController::new(),
            rate_limit: rate_limit::RateLimitController::new(&CONFIG.grpc.trust_host),
            identity: Mutex::new(identity),
            db,
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
            .serve_with_shutdown(CONFIG.address.clone().parse().unwrap(), async {
                if tokio::signal::ctrl_c().await.is_err() {
                    tracing::warn!("graceful_shutdown");
                }
            })
            .await
            .unwrap();
    }
}
