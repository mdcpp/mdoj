#[cfg(feature = "ssr")]
use std::sync::OnceLock;

use cfg_if::cfg_if;
use leptos::*;
use serde::{Deserialize, Serialize};

use crate::error::*;
#[cfg(feature = "ssr")]
static CONFIG: OnceLock<Config> = OnceLock::new();

#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
pub struct Config {
    #[serde(default = "default_frontend_config")]
    pub frontend: FrontendConfig,
    #[serde(default = "default_backend_config")]
    pub backend: BackendConfig,
}

#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
pub struct FrontendConfig {
    pub api_server: String,
    pub image_providers: Vec<String>,
}

#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
pub struct BackendConfig {}

fn default_frontend_config() -> FrontendConfig {
    FrontendConfig {
        api_server: "http://0.0.0.0:8081".to_owned(),
        image_providers: vec!["https://i.imgur.com".to_owned()],
    }
}

fn default_backend_config() -> BackendConfig {
    BackendConfig {}
}

#[cfg(feature = "ssr")]
pub async fn init_server_config() -> Result<()> {
    let config = load_server_config().await?;
    CONFIG.set(config);
    Ok(())
}

#[cfg(feature = "ssr")]
async fn load_server_config() -> Result<Config> {
    use std::env::var;

    use tokio::{fs, io::AsyncReadExt};

    const DEFAULT_CONFIG_PATH: &str = "config.toml";
    let config_path = var("CONFIG_PATH")
        .ok()
        .unwrap_or(DEFAULT_CONFIG_PATH.into());

    if fs::metadata(&config_path).await.is_ok() {
        let mut buf = String::new();
        fs::File::open(&config_path)
            .await?
            .read_to_string(&mut buf)
            .await?;
        return Ok(toml::from_str(&buf).expect("malformed config file"));
    }
    let default_config = Config {
        frontend: default_frontend_config(),
        backend: default_backend_config(),
    };
    let default_toml = toml::to_string_pretty(&default_config)
        .expect("Cannot generate default config");
    fs::write(&config_path, default_toml).await?;
    Ok(default_config)
}

#[server]
async fn get_frontend_config() -> Result<FrontendConfig, ServerFnError> {
    Ok(CONFIG
        .get()
        .map(|c| c.frontend.clone())
        .expect("server config is not init!"))
}

pub async fn frontend_config() -> Result<FrontendConfig> {
    cfg_if! { if #[cfg(feature = "ssr")] {
        Ok(get_frontend_config().await?)
    } else {
        use gloo::storage::{SessionStorage, Storage};
        const SERVER_CONFIG_KEY: &str = "server_config";
        if let Ok(config) = SessionStorage::get(SERVER_CONFIG_KEY) {
            Ok(config)
        } else {
            let config= get_frontend_config().await?;
            SessionStorage::set(SERVER_CONFIG_KEY, Some(config.clone())).map_err(|_|ErrorKind::Browser)?;
            Ok(config)
        }
    }}
}

#[cfg(feature = "ssr")]
pub fn backend_config() -> &'static BackendConfig {
    CONFIG
        .get()
        .map(|c| &c.backend)
        .expect("server config is not init!")
}
