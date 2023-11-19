use std::sync::Arc;

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

pub struct Server {
    pub token: Arc<token::TokenController>,
    pub submit: submit::SubmitController,
    pub dup: DupController,
}

impl Server {
    pub async fn start() {
        let config = CONFIG.get().unwrap();

        log::info!("Constructing server...");

        let server = Arc::new(Server {
            token: token::TokenController::new(),
            submit: submit::SubmitController::new().await.unwrap(),
            dup: DupController::new(),
        });

        transport::Server::builder()
            .accept_http1(true)
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
