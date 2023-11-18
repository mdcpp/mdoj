use std::{path::PathBuf, sync::Arc};

use ring::rand::SystemRandom;
use serde::{Deserialize, Serialize};
use tokio::{fs, io::AsyncReadExt, sync::OnceCell};

pub static CONFIG: OnceCell<GlobalConfig> = OnceCell::const_new();

const CONFIG_PATH: &'static str = "config.toml";

#[derive(Serialize, Deserialize, Debug)]
pub struct GlobalConfig {
    #[serde(default = "default_bind_address")]
    pub bind_address: String,
    #[serde(default)]
    pub database: Database,
    #[serde(default)]
    pub log_level: usize,
    #[serde(default = "default_judger")]
    pub judger: Vec<Arc<Judger>>,
    pub judger_secret: Option<String>,
    #[serde(default)]
    grpc: GrpcOption,
}
fn default_bind_address() -> String {
    "0.0.0.0:8081".to_string()
}
fn default_judger() -> Vec<Arc<Judger>> {
    vec![Arc::new(Judger {
        uri: "http://127.0.0.1:8080".to_owned(),
        pem: None,
        domain: None,
    })]
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Judger {
    pub uri: String,
    pub pem: Option<PathBuf>,
    pub domain: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GrpcOption {
    pub trust_x_forwarded_for: bool,
}

impl Default for GrpcOption {
    fn default() -> Self {
        Self {
            trust_x_forwarded_for: false,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Database {
    pub uri: String,
    pub salt: String,
}

impl Default for Database {
    fn default() -> Self {
        let rng = SystemRandom::new();
        Self {
            uri: "sqlite://test.sqlite".to_owned(),
            salt: "be sure to change it".to_owned(),
        }
    }
}

pub async fn init() {
    if fs::metadata(CONFIG_PATH).await.is_ok() {
        let mut buf = Vec::new();
        let mut config = fs::File::open(CONFIG_PATH)
            .await
            .expect(&format!("Cannot found ,{}", CONFIG_PATH));
        config.read_to_end(&mut buf).await.unwrap();
        let config =
            std::str::from_utf8(&buf).expect("Config file may container non-utf8 character");
        let config: GlobalConfig = toml::from_str(config).unwrap();
        CONFIG.set(config).ok();
    } else {
        println!("Unable to find {}, generating default config", CONFIG_PATH);
        let config: GlobalConfig = toml::from_str("").unwrap();

        let config_txt = toml::to_string(&config).unwrap();
        fs::write(CONFIG_PATH, config_txt).await.unwrap();
        CONFIG.set(config).ok();
    }
}

#[cfg(test)]
mod test {
    use super::{init, CONFIG};

    #[tokio::test]
    async fn default() {
        init().await;
        assert!(CONFIG.get().is_some());
    }
}
