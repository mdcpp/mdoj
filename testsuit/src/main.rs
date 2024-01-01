pub mod case;
pub mod client;
pub mod constant;
pub mod grpc;
pub mod user;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    env_logger::builder()
        .filter_module("testsuit", log::LevelFilter::Trace)
        .init();

    user::run().await;
}
