mod config;
mod error;
mod filesystem;
mod language;
mod sandbox;
mod server;

pub use config::CONFIG;

use grpc::judger::judger_server::JudgerServer;
use server::Server;

type Result<T> = std::result::Result<T, error::Error>;

#[tokio::main]
async fn main() {
    // FIXME: use CONFIG for logging
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Trace)
        .try_init()
        .ok();

    let server = Server::new().await.unwrap();

    tonic::transport::Server::builder()
        .add_service(JudgerServer::new(server))
        .serve(CONFIG.address)
        .await
        .unwrap();
}
