use controller::token::TokenController;

pub mod controller;
pub mod endpoint;
pub mod grpc;
pub mod init;

#[derive(Default)]
pub struct Server {
    pub controller: TokenController,
}

#[tokio::main]
async fn main() {
    // init::new().await;

    // let config = CONFIG.get().unwrap();
    // let addr = config.runtime.bind.parse().unwrap();

    // log::info!("Server started");

    // let server = Server{ controller: TokenController::new() };

    // Server::builder()
    //     .add_service(server)
    //     .serve(addr)
    //     .await
    //     .unwrap();
}
