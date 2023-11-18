use std::{net::SocketAddr, path::PathBuf, sync::Arc};

use serde::{Deserialize, Serialize};
use tokio::{fs, io::AsyncReadExt, sync::OnceCell};

pub static CONFIG: OnceCell<GlobalConfig> = OnceCell::const_new();

const CONFIG_PATH: &'static str = "config.toml";

#[derive(Serialize, Deserialize, Debug)]
pub struct GlobalConfig {
    pub bind_address: SocketAddr,
    #[serde(default)]
    pub database: Database,
    #[serde(default)]
    pub log_level: usize,
    #[serde(default)]
    pub judger: Vec<Arc<Judger>>,
    #[serde(default)]
    grpc: GrpcOption,
    // pub reverse_proxy: Vec<ReverseProxy>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Judger {
    pub uri: SocketAddr,
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
}

impl Default for Database {
    fn default() -> Self {
        Self {
            uri: "sqlite://test.sqlite".to_owned(),
        }
    }
}

pub async fn init() {
    let mut buf = Vec::new();

    let config = if fs::metadata(CONFIG_PATH).await.is_ok() {
        let mut config = fs::File::open(CONFIG_PATH)
            .await
            .expect(&format!("Cannot found ,{}", CONFIG_PATH));
        config.read_to_end(&mut buf).await.unwrap();
        std::str::from_utf8(&buf).expect("Config file may container non-utf8 character")
    } else {
        println!("using default config");
        ""
    };

    let config: GlobalConfig = toml::from_str(config).unwrap();

    CONFIG.set(config).ok();
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
