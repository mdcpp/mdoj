use self::config::GlobalConfig;

pub mod config;
pub mod db;
pub mod logger;

pub async fn new() -> GlobalConfig {
    let config = config::init().await;
    logger::init(&config);
    db::init(&config).await;
    config
}
