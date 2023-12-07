use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tokio::{fs, io::AsyncReadExt};

static CONFIG_PATH: &str = "config/config.toml";

#[derive(Serialize, Deserialize, Debug)]
pub struct GlobalConfig {
    #[serde(default = "default_bind_address")]
    pub bind_address: String,
    #[serde(default)]
    pub database: Database,
    #[serde(default)]
    pub log_level: usize,
    #[serde(default = "default_judger")]
    pub judger: Vec<Judger>,
    #[serde(default)]
    pub grpc: GrpcOption,
}
fn default_bind_address() -> String {
    "0.0.0.0:8081".to_string()
}
fn default_judger() -> Vec<Judger> {
    vec![Judger {
        name: "http://127.0.0.1:8080".to_owned(),
        secret: None,
        judger_type: JudgerType::Static,
    }]
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum JudgerType {
    Docker,
    Static,
}

impl Default for JudgerType {
    fn default() -> Self {
        Self::Static
    }
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Judger {
    pub name: String,
    pub secret: Option<String>,
    #[serde(rename = "type")]
    pub judger_type: JudgerType,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GrpcOption {
    pub trust_x_forwarded_for: bool,
    pub public_pem: PathBuf,
    pub private_pem: PathBuf,
}

impl Default for GrpcOption {
    fn default() -> Self {
        Self {
            trust_x_forwarded_for: false,
            public_pem: "cert.pem".into(),
            private_pem: "key.pem".into(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Database {
    pub path: String,
    pub salt: String,
}

impl Default for Database {
    fn default() -> Self {
        Self {
            path: "database/backend.sqlite".to_owned(),
            salt: "be sure to change it".to_owned(),
        }
    }
}

pub async fn init() -> GlobalConfig {
    if fs::metadata(CONFIG_PATH).await.is_ok() {
        let mut buf = Vec::new();
        let mut config = fs::File::open(CONFIG_PATH)
            .await
            .unwrap_or_else(|_| panic!("Cannot found ,{}", CONFIG_PATH));
        config.read_to_end(&mut buf).await.unwrap();
        let config =
            std::str::from_utf8(&buf).expect("Config file may container non-utf8 character");
        let config: GlobalConfig = toml::from_str(config).unwrap();
        config
    } else {
        println!("Unable to find {}, generating default config", CONFIG_PATH);
        let config: GlobalConfig = toml::from_str("").unwrap();

        let config_txt = toml::to_string(&config).unwrap();
        fs::write(CONFIG_PATH, config_txt).await.unwrap();

        println!(
            "Config generated, please edit {} before restart",
            CONFIG_PATH
        );
        println!("Finished, exiting...");
        std::process::exit(0);
    }
}

#[cfg(test)]
mod test {
    use super::init;

    #[tokio::test]
    async fn default() {
        init().await;
    }
}
