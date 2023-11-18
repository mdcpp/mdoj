use std::{net::SocketAddr, path::PathBuf, sync::Arc};

use ring::rand::{generate, SystemRandom};
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
    // pub reverse_proxy: Vec<ReverseProxy>,
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
    pub cert: String,
    pub mode: GrpcMode,
}

impl Default for GrpcOption {
    fn default() -> Self {
        Self {
            trust_x_forwarded_for: false,
            cert: "path-to-cert".to_owned(),
            mode: GrpcMode::Unsecure,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum GrpcMode {
    Unsecure,
    Secure,
    Web,
}

// #[derive(Serialize, Deserialize, Debug)]
// pub struct ReverseProxy{
//     pub address: String,
// }

// #[derive(Serialize, Deserialize, Debug)]
// pub struct Sandbox {
//     address: String,
//     port: u16,
//     memory: u64,
//     cpu_weight: usize,
// }

#[derive(Serialize, Deserialize, Debug)]
pub struct Database {
    pub uri: String,
    pub salt: String,
}

impl Default for Database {
    fn default() -> Self {
        let rng = SystemRandom::new();
        let salt: [u8; 10] = generate(&rng).unwrap().expose();
        Self {
            uri: "sqlite://test.sqlite".to_owned(),
            salt: String::from_utf8_lossy(&salt).to_string(),
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
