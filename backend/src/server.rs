use std::sync::Arc;

use sea_orm::DatabaseConnection;
use tonic::transport::{self, Identity, ServerTlsConfig};
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
    },
};

const MAX_FRAME_SIZE: u32 = 1024 * 1024 * 8;

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
    _otel_guard: OtelGuard,
}

impl Server {
    pub async fn new() -> Arc<Self> {
        let config = config::init().await;

        let otel_guard = logger::init(&config);
        let span = span!(Level::INFO, "server_construct");
        let crypto = crypto::CryptoController::new(&config, &span);
        let db = Arc::new(
            crate::init::db::init(&config.database, &crypto, &span)
                .in_current_span()
                .await,
        );

        Arc::new(Server {
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
            db,
            _otel_guard: otel_guard,
        })
    }
    pub async fn start(self: Arc<Self>) {
        let mut identity = None;

        if self.config.grpc.private_pem.is_some() {
            let private_pem = self.config.grpc.private_pem.as_ref().unwrap();
            let public_pem = self
                .config
                .grpc
                .public_pem
                .as_ref()
                .expect("public pem should set if private pem is set");

            let cert = std::fs::read_to_string(public_pem).expect("cannot read public pem");
            let key = std::fs::read_to_string(private_pem).expect("cannot read private pem");

            identity = Some(Identity::from_pem(cert, key));
        }

        let server = match identity {
            Some(identity) => transport::Server::builder()
                .tls_config(ServerTlsConfig::new().identity(identity))
                .unwrap(),
            None => transport::Server::builder().accept_http1(true),
        };
        server
            .max_frame_size(Some(MAX_FRAME_SIZE))
            .add_service(tonic_web::enable(ProblemSetServer::new(self.clone())))
            .add_service(tonic_web::enable(EducationSetServer::new(self.clone())))
            .add_service(tonic_web::enable(UserSetServer::new(self.clone())))
            .add_service(tonic_web::enable(TokenSetServer::new(self.clone())))
            .add_service(tonic_web::enable(ContestSetServer::new(self.clone())))
            .add_service(tonic_web::enable(TestcaseSetServer::new(self.clone())))
            .add_service(tonic_web::enable(SubmitSetServer::new(self.clone())))
            .add_service(tonic_web::enable(ChatSetServer::new(self.clone())))
            .add_service(tonic_web::enable(AnnouncementSetServer::new(self.clone())))
            .serve(self.config.bind_address.clone().parse().unwrap())
            .await
            .unwrap();
    }
}
