use std::sync::Arc;

use tonic::transport;

use crate::{
    controller::{duplicate::DupController, *},
    grpc::backend::{
        contest_set_server::ContestSetServer, problem_set_server::ProblemSetServer,
        testcase_set_server::TestcaseSetServer,
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
            .add_service(ProblemSetServer::new(server.clone()))
            .add_service(ContestSetServer::new(server.clone()))
            .add_service(TestcaseSetServer::new(server))
            .serve(config.bind_address.parse().unwrap())
            .await
            .unwrap();
    }
}
