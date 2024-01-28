use std::path::Path;

use tokio::fs;

use super::config::CONFIG;

pub async fn init() {
    let config = CONFIG.get().unwrap();
    let path: &Path = config.runtime.temp.as_ref();
    fs::create_dir_all(path).await.unwrap();
}
