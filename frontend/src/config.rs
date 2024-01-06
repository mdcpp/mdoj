use anyhow::{anyhow, Result};
use cfg_if::cfg_if;
use leptos::*;
use leptos_use::storage::{use_local_storage, JsonCodec};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use tonic::{IntoRequest, Request};

#[cfg(feature = "ssr")]
static CONFIG: OnceLock<GlobalConfig> = OnceLock::new();

#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
pub struct GlobalConfig {
    #[serde(default = "default_backend")]
    pub backend: String,
    #[serde(default = "default_image_providers")]
    pub image_providers: Vec<String>,
}

fn default_backend() -> String {
    "http://0.0.0.0:8081".to_owned()
}

fn default_image_providers() -> Vec<String> {
    vec!["https://i.imgur.com".to_owned()]
}

#[cfg(feature = "ssr")]
pub async fn init() -> Result<()> {
    let config = load_server_config().await?;
    CONFIG
        .set(config)
        .map_err(|_| anyhow!("cannot multiple init"))?;
    Ok(())
}

#[cfg(feature = "ssr")]
async fn load_server_config() -> Result<GlobalConfig> {
    use tokio::{fs, io::AsyncReadExt};

    const CONFIG_DIR: &str = "./config";
    const CONFIG_FILE_PATH: &str = "./config/frontend.toml";

    if fs::metadata(CONFIG_FILE_PATH).await.is_ok() {
        let mut buf = String::new();
        fs::File::open(CONFIG_FILE_PATH)
            .await?
            .read_to_string(&mut buf)
            .await?;
        return Ok(toml::from_str(&buf)?);
    }
    let default_toml = toml::to_string_pretty(&GlobalConfig {
        backend: default_backend(),
        image_providers: default_image_providers(),
    })?;
    fs::create_dir_all(CONFIG_DIR).await?;
    fs::write(CONFIG_FILE_PATH, default_toml).await?;
    return Err(anyhow!("Please edit config"));
}

#[server]
async fn get_server_config() -> Result<GlobalConfig, ServerFnError> {
    return Ok(CONFIG.get().cloned().unwrap());
}

pub async fn server_config() -> Result<GlobalConfig> {
    cfg_if! { if #[cfg(feature = "ssr")] {
        Ok(get_server_config().await.map_err(|_|anyhow!("Cannot get config from server"))?)
    } else {
        use gloo_storage::{LocalStorage, Storage};
        const SERVER_CONFIG_KEY: &str = "server_config";
        if let Ok(config) = LocalStorage::get(SERVER_CONFIG_KEY) {
            Ok(config)
        } else {
            let config= get_server_config()
                .await
                .map_err(|_| anyhow!("Cannot get config from server"))?;
            LocalStorage::set(SERVER_CONFIG_KEY, Some(config.clone()))?;
            Ok(config)
        }
    }}
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Deserialize, Serialize)]
pub struct LoginInfo {
    pub token: String,
    // TODO
    pub permission: u32,
    pub expiry: u32,
}

pub fn use_login_info() -> (
    Signal<Option<LoginInfo>>,
    WriteSignal<Option<LoginInfo>>,
    impl Fn() + Clone,
) {
    use_local_storage::<Option<LoginInfo>, JsonCodec>("login_info")
}

pub fn with_token<T>(req: impl IntoRequest<T>) -> Result<Request<T>> {
    let mut req = req.into_request();
    let (login_info, ..) = use_login_info();
    let token = login_info()
        .ok_or_else(|| anyhow!("You need login first"))?
        .token;
    req.metadata_mut().insert("token", token.parse()?);
    Ok(req)
}
