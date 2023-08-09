pub mod config;
pub mod db;
pub mod logger;

pub async fn new() {
    config::init().await;
    logger::init();
    db::init().await;
}
