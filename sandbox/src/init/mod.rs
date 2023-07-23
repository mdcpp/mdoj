pub mod cgroup;
pub mod check;
pub mod config;
pub mod logger;

pub async fn new() {
    config::init().await;
    logger::init();
    cgroup::init();
    check::init();
}
