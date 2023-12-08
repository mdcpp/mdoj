use self::{config::GlobalConfig, logger::OtelGuard};

pub mod config;
pub mod db;
pub mod logger;

pub async fn new() -> (GlobalConfig, OtelGuard) {
    let config = config::init().await;
    let olp_guard = logger::init(&config);
    db::init(&config).await;
    (config, olp_guard)
}
