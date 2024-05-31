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
        .filter_level(log::LevelFilter::Debug)
        .try_init()
        .ok();

    let default_panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        log::error!("something panic, exiting...");
        default_panic(info);
        std::process::exit(1);
    }));

    #[cfg(debug_assertions)]
    log::warn!("running debug build");

    let server = Server::new().await.unwrap();

    tonic::transport::Server::builder()
        .add_service(JudgerServer::new(server))
        .serve(CONFIG.address)
        .await
        .unwrap();
}
