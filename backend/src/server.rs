use std::sync::Arc;

use tonic::transport;
use tracing::{span, Instrument, Level};

use crate::{
    controller::*,
    grpc::backend::{
        chat_set_server::ChatSetServer, contest_set_server::ContestSetServer,
        education_set_server::EducationSetServer, problem_set_server::ProblemSetServer,
        submit_set_server::SubmitSetServer, testcase_set_server::TestcaseSetServer,
        token_set_server::TokenSetServer, user_set_server::UserSetServer,
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
    config: GlobalConfig,
    _otel_guard: OtelGuard,
}

impl Server {
    pub async fn new() -> Arc<Self> {
        let config = config::init().await;

        let otel_guard = logger::init(&config);

        let span = span!(Level::INFO, "server_construct");

        let crypto = crypto::CryptoController::new(&config, &span);
        crate::init::db::init(&config.database, &crypto, &span)
            .in_current_span()
            .await;

        let judger = judger::JudgerController::new(config.judger.clone(), &span)
            .in_current_span()
            .await
            .unwrap();

        Arc::new(Server {
            token: token::TokenController::new(&span),
            judger: Arc::new(judger),
            dup: duplicate::DupController::new(&span),
            crypto,
            metrics: metrics::MetricsController::new(&otel_guard.meter_provider),
            imgur: imgur::ImgurController::new(&config.imgur),
            config,
            _otel_guard: otel_guard,
        })
    }
    #[cfg(not(feature="testsuit"))]
    pub async fn start(self: Arc<Self>) {
        transport::Server::builder()
            .accept_http1(true)
            .max_frame_size(Some(MAX_FRAME_SIZE))
            .add_service(tonic_web::enable(ProblemSetServer::new(self.clone())))
            .add_service(tonic_web::enable(EducationSetServer::new(self.clone())))
            .add_service(tonic_web::enable(UserSetServer::new(self.clone())))
            .add_service(tonic_web::enable(TokenSetServer::new(self.clone())))
            .add_service(tonic_web::enable(ContestSetServer::new(self.clone())))
            .add_service(tonic_web::enable(TestcaseSetServer::new(self.clone())))
            .add_service(tonic_web::enable(SubmitSetServer::new(self.clone())))
            .add_service(tonic_web::enable(ChatSetServer::new(self.clone())))
            .serve(self.config.bind_address.clone().parse().unwrap())
            .await
            .unwrap();
    }
    #[cfg(feature="testsuit")]
    pub async fn start(self: Arc<Self>) {
        transport::Server::builder()
            .accept_http1(true)
            .max_frame_size(Some(MAX_FRAME_SIZE))
            .add_service(ProblemSetServer::new(self.clone()))
            .add_service(EducationSetServer::new(self.clone()))
            .add_service(UserSetServer::new(self.clone()))
            .add_service(TokenSetServer::new(self.clone()))
            .add_service(ContestSetServer::new(self.clone()))
            .add_service(TestcaseSetServer::new(self.clone()))
            .add_service(SubmitSetServer::new(self.clone()))
            .add_service(ChatSetServer::new(self.clone()))
            .serve(self.config.bind_address.clone().parse().unwrap())
            .await
            .unwrap();
    }
}
