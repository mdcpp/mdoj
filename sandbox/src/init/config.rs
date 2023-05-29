use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use tokio::{fs, io::AsyncReadExt};

pub static CONFIG: OnceCell<GlobalConfig> = OnceCell::new();

const CONFIG_PATH: &'static str = "config.toml";

#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct GlobalConfig {
    #[serde(default)]
    pub runtime: Runtime,
    #[serde(default)]
    pub nsjail: Nsjail,
    #[serde(default)]
    pub plugin: Plugin,
    #[serde(default)]
    pub log_level: usize,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Nsjail {
    pub runtime: String,
    pub rootless: bool,
    pub log: String,
}

impl Default for Nsjail {
    fn default() -> Self {
        Self {
            runtime: "nsjail-sys/nsjail/nsjail".to_owned(),
            rootless: false,
            log: "/dev/null".to_owned(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Plugin {
    pub path: String,
}

impl Default for Plugin {
    fn default() -> Self {
        Self {
            path: "plugins".to_owned(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Runtime {
    pub temp: String,
    pub available_memory: u64,
    pub bind: String,
}

impl Default for Runtime {
    fn default() -> Self {
        Self {
            temp: "temp".to_owned(),
            available_memory: 1073741824,
            bind: "0.0.0.0:8080".to_owned(),
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
