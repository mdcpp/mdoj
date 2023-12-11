use std::sync::Arc;

use tonic::transport;
use tracing::{span, Instrument, Level};

use crate::{
    controller::*,
    grpc::backend::{
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
    config: GlobalConfig,
    otel_guard: OtelGuard,
}

impl Server {
    pub async fn new() -> Arc<Self> {
        let span = span!(Level::INFO, "server_construct");

        let config = config::init().await;

        let crypto = crypto::CryptoController::new(&config, &span);
        crate::init::db::init(&config.database, &crypto).await;

        let otel_guard = logger::init(&config);

        let config4 = config.judger.clone();

        let submit = judger::JudgerController::new(config4, &span)
            .in_current_span()
            .await
            .unwrap();

        Arc::new(Server {
            token: token::TokenController::new(&span),
            judger: Arc::new(submit),
            dup: duplicate::DupController::new(&span),
            crypto,
            metrics: metrics::MetricsController::new(&otel_guard.meter_provider),
            config,
            // identity,
            otel_guard,
        })
    }
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
            .serve(self.config.bind_address.clone().parse().unwrap())
            .instrument(tracing::info_span!("server_serve"))
            .await
            .unwrap();
    }
}
