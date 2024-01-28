pub mod config;
pub mod db;
pub mod error;
pub mod logger;

// pub async fn new() -> (GlobalConfig, OtelGuard) {
//     let config = config::init().await;
//     let olp_guard = logger::init(&config);
//     db::init(&config.database).await;
//     (config, olp_guard)
// }
