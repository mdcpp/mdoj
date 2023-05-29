pub mod check;
pub mod config;
pub mod logger;

pub async fn new() {
    config::init().await;
    logger::init();
    check::init();
}
