use serde::{Deserialize, Serialize};
use tokio::{fs, io::{AsyncReadExt, AsyncWriteExt}, sync::OnceCell};
use anyhow::Result;

const CONFIG_FILE_PATH:&str="./config/config.toml";
static CONFIG:OnceCell<GlobalConfig>=OnceCell::const_new();


#[derive(Debug, Deserialize, Serialize)]
pub struct GlobalConfig {
    #[serde(default = "default_backend")]
    pub backend: String,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self { backend: default_backend() }
    }
}

fn default_backend() -> String {
    "0.0.0.0:8081".to_owned()
}

pub async fn init()->Result<()> {
    let config=load_config().await?;
    CONFIG.set(config)?;
    Ok(())
}

async fn load_config()->Result<GlobalConfig> {
    if fs::metadata(CONFIG_FILE_PATH).await.is_ok() {
        let mut buf=String::new();
        fs::File::open(CONFIG_FILE_PATH).await?.read_to_string(&mut buf).await?;
        return Ok(toml::from_str(&buf)?)
    }
    let default=GlobalConfig::default();
    let mut file=fs::File::create(CONFIG_FILE_PATH).await?;
    let default_toml=toml::to_string_pretty(&default)?;
    file.write_all(default_toml.as_bytes()).await?;
    Ok(default)
}
