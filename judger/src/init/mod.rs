use thiserror::Error;

pub mod cgroup;
pub mod check;
pub mod config;
pub mod logger;
pub mod volumn;

pub async fn new() {
    config::init().await;
    logger::init();
    cgroup::init();
    check::init();
    volumn::init().await;
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("unmeet system requirements")]
    SystemIncapable,
    #[error("Fail to load Langs `{0}`")]
    Langs(#[from] crate::langs::InitError),
}
