use std::sync::Arc;

use tokio::fs;
use tonic::transport;

use crate::{
    controller::{duplicate::DupController, *},
    grpc::backend::{
        contest_set_server::ContestSetServer, education_set_server::EducationSetServer,
        problem_set_server::ProblemSetServer, testcase_set_server::TestcaseSetServer,
        token_set_server::TokenSetServer, user_set_server::UserSetServer,
    },
    init::config::CONFIG,
};

const MAX_FRAME_SIZE: u32 = 1024 * 1024 * 8;

pub struct Server {
    pub token: Arc<token::TokenController>,
    pub submit: submit::SubmitController,
    pub dup: DupController,
}

impl Server {
    pub async fn start() {
        let config = CONFIG.get().unwrap();

        log::info!("Loading TLS certificate...");
        let cert = fs::read_to_string(&config.grpc.public_pem).await.unwrap();
        let key = fs::read_to_string(&config.grpc.private_pem).await.unwrap();
        let identity = transport::Identity::from_pem(cert, key);

        log::info!("Constructing server...");

        let server = Arc::new(Server {
            token: token::TokenController::new(),
            submit: submit::SubmitController::new().await.unwrap(),
            dup: DupController::default(),
        });

        transport::Server::builder()
            .tls_config(transport::ServerTlsConfig::new().identity(identity))
            .unwrap()
            .max_frame_size(Some(MAX_FRAME_SIZE))
            .add_service(tonic_web::enable(ProblemSetServer::new(server.clone())))
            .add_service(tonic_web::enable(EducationSetServer::new(server.clone())))
            .add_service(tonic_web::enable(UserSetServer::new(server.clone())))
            .add_service(tonic_web::enable(TokenSetServer::new(server.clone())))
            .add_service(tonic_web::enable(ContestSetServer::new(server.clone())))
            .add_service(tonic_web::enable(TestcaseSetServer::new(server)))
            .serve(config.bind_address.parse().unwrap())
            .await
            .unwrap();
    }
}
