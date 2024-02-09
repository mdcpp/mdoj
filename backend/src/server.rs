use std::sync::Arc;

use http::header::HeaderName;
use sea_orm::DatabaseConnection;
use spin::Mutex;
use tonic::transport::{self, Identity, ServerTlsConfig};
use tonic_web::GrpcWebLayer;
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use tracing::{span, Instrument, Level};

use crate::{
    controller::*,
    grpc::backend::{
        announcement_set_server::AnnouncementSetServer, chat_set_server::ChatSetServer,
        contest_set_server::ContestSetServer, education_set_server::EducationSetServer,
        problem_set_server::ProblemSetServer, submit_set_server::SubmitSetServer,
        testcase_set_server::TestcaseSetServer, token_set_server::TokenSetServer,
        user_set_server::UserSetServer,
    },
    init::{
        config::{self, GlobalConfig},
        logger::{self, OtelGuard},
        Error,
    },
};

const MAX_FRAME_SIZE: u32 = 1024 * 1024 * 8;

/// A wrapper to launch server
///
/// [`Server`] doesn't hold state
pub struct Server {
    pub token: Arc<token::TokenController>,
    pub judger: Arc<judger::JudgerController>,
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
                judger::JudgerController::new(config.judger.clone(), db.clone(), &span)
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
        let cors = CorsLayer::new()
            .allow_headers([HeaderName::from_static("token")])
            .allow_origin(AllowOrigin::mirror_request())
            .allow_methods(Any);

        let server = match self.identity.lock().take() {
            Some(identity) => transport::Server::builder()
                .tls_config(ServerTlsConfig::new().identity(identity))
                .unwrap(),
            None => transport::Server::builder().accept_http1(true),
        };

        server
            .layer(cors)
            .layer(GrpcWebLayer::new())
            .max_frame_size(Some(MAX_FRAME_SIZE))
            .add_service(ProblemSetServer::new(self.clone()))
            .add_service(EducationSetServer::new(self.clone()))
            .add_service(UserSetServer::new(self.clone()))
            .add_service(TokenSetServer::new(self.clone()))
            .add_service(ContestSetServer::new(self.clone()))
            .add_service(TestcaseSetServer::new(self.clone()))
            .add_service(SubmitSetServer::new(self.clone()))
            .add_service(ChatSetServer::new(self.clone()))
            .add_service(AnnouncementSetServer::new(self.clone()))
            .serve(self.config.bind_address.clone().parse().unwrap())
            .await
            .unwrap();
    }
}
