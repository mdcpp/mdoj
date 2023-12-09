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
        self,
        config::{self, GlobalConfig},
        logger::{self, OtelGuard},
    },
};

const MAX_FRAME_SIZE: u32 = 1024 * 1024 * 8;

pub struct Server {
    pub token: Arc<token::TokenController>,
    pub submit: Arc<judger::JudgerController>,
    pub dup: duplicate::DupController,
    pub crypto: crypto::CryptoController,
    pub metrics: metrics::MetricsController,
    config: GlobalConfig,
    // identity: transport::Identity,
    otel_guard: OtelGuard,
}

impl Server {
    pub async fn new() -> Arc<Self> {
        let config = config::init().await;
        let otel_guard = logger::init(&config);

        let config1 = config.database.clone();
        // let config2 = config.grpc.public_pem.clone();
        // let config3 = config.grpc.private_pem.clone();
        let config4 = config.judger.clone();

        let span = span!(Level::INFO, "server_construct");
        let span1 = span.clone();

        let (_, submit) = tokio::try_join!(
            tokio::spawn(
                async move { init::db::init(&config1).await }
                    .instrument(span!(parent:span.clone(),Level::INFO,"construct_database"))
            ),
            // tokio::spawn(
            //     async move { fs::read_to_string(&config2).await }
            //         .instrument(span!(parent:span.clone(),Level::INFO,"load_tls"))
            // ),
            // tokio::spawn(
            //     async move { fs::read_to_string(&config3).await }
            //         .instrument(span!(parent:span.clone(),Level::INFO,"load_tls"))
            // ),
            tokio::spawn(async move { judger::JudgerController::new(config4, &span1).await })
        )
        .unwrap();

        // let identity = transport::Identity::from_pem(
        //     cert.expect("public key.pem not found"),
        //     key.expect("privite key.pem not found"),
        // );

        Arc::new(Server {
            token: token::TokenController::new(&span),
            submit: Arc::new(submit.unwrap()),
            dup: duplicate::DupController::new(&span),
            crypto: crypto::CryptoController::new(&config, &span),
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
