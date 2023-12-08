use std::sync::Arc;

use tokio::fs;
use tonic::transport;

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
    pub submit: judger::SubmitController,
    pub dup: duplicate::DupController,
    pub crypto: crypto::CryptoController,
    config: GlobalConfig,
    identity: transport::Identity,
    _otp_guard: OtelGuard,
}

impl Server {
    pub async fn new() -> Arc<Self> {
        let config = config::init().await;

        init::db::init(&config).await;
        let otp_guard = logger::init(&config);

        log::info!("Loading TLS certificate...");
        let cert = fs::read_to_string(&config.grpc.public_pem)
            .await
            .expect("public key.pem not found");
        let key = fs::read_to_string(&config.grpc.private_pem)
            .await
            .expect("privite key.pem not found");
        let identity = transport::Identity::from_pem(cert, key);

        log::info!("Constructing server...");

        Arc::new(Server {
            token: token::TokenController::new(),
            submit: judger::SubmitController::new(&config).await.unwrap(),
            dup: duplicate::DupController::default(),
            crypto: crypto::CryptoController::new(&config),
            config,
            identity,
            _otp_guard: otp_guard,
        })
    }
    pub async fn start(self: Arc<Self>) {
        transport::Server::builder()
            // .accept_http1(true)
            .tls_config(transport::ServerTlsConfig::new().identity(self.identity.clone()))
            .unwrap()
            .max_frame_size(Some(MAX_FRAME_SIZE))
            .add_service(tonic_web::enable(ProblemSetServer::new(self.clone())))
            .add_service(tonic_web::enable(EducationSetServer::new(self.clone())))
            .add_service(tonic_web::enable(UserSetServer::new(self.clone())))
            .add_service(tonic_web::enable(TokenSetServer::new(self.clone())))
            .add_service(tonic_web::enable(ContestSetServer::new(self.clone())))
            .add_service(tonic_web::enable(TestcaseSetServer::new(self.clone())))
            .add_service(tonic_web::enable(SubmitSetServer::new(self.clone())))
            .serve(self.config.bind_address.clone().parse().unwrap())
            .await
            .unwrap();
    }
}
