use serde::{Deserialize, Serialize};
use tokio::{fs, io::AsyncReadExt, sync::OnceCell};

pub static CONFIG: OnceCell<GlobalConfig> = OnceCell::const_new();

const CONFIG_PATH: &'static str = "config.toml";

#[derive(Serialize, Deserialize, Debug)]
pub struct GlobalConfig {
    #[serde(default)]
    pub database: Database,
    #[serde(default)]
    pub log_level: usize,
    #[serde(default)]
    pub sandbox:Vec<Sandbox>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Sandbox{
    address: String,
    port:u16,
    memory: i64,
    cpu_weight: usize
}

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
