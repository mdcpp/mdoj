#[cfg(feature = "ssr")]
use std::sync::OnceLock;

use cfg_if::cfg_if;
use leptos::*;
use leptos_use::{utils::JsonCodec, *};
use serde::{Deserialize, Serialize};
use tonic::{IntoRequest, Request};

use crate::{error::*, grpc};
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
    CONFIG.set(config).expect("Already init!");
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
        return Ok(toml::from_str(&buf).expect("malformed config file"));
    }
    let default_config = GlobalConfig {
        backend: default_backend(),
        image_providers: default_image_providers(),
    };
    let default_toml = toml::to_string_pretty(&default_config)
        .expect("Cannot generate default config");
    fs::create_dir_all(CONFIG_DIR).await?;
    fs::write(CONFIG_FILE_PATH, default_toml).await?;
    Ok(default_config)
}

#[server]
async fn get_server_config() -> Result<GlobalConfig, ServerFnError> {
    return Ok(CONFIG.get().cloned().unwrap());
}

pub async fn server_config() -> Result<GlobalConfig> {
    cfg_if! { if #[cfg(feature = "ssr")] {
        Ok(get_server_config().await?)
    } else {
        use gloo::storage::{LocalStorage, Storage};
        const SERVER_CONFIG_KEY: &str = "server_config";
        if let Ok(config) = LocalStorage::get(SERVER_CONFIG_KEY) {
            Ok(config)
        } else {
            let config= get_server_config()
                .await
                .map_err(|_| ErrorKind::ServerError(ServerErrorKind::ServerFn))?;
            LocalStorage::set(SERVER_CONFIG_KEY, Some(config.clone())).map_err(|_|ErrorKind::Browser)?;
            Ok(config)
        }
    }}
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Deserialize, Serialize)]
pub struct Token {
    pub token: String,
    pub role: grpc::Role,
}

pub fn use_token() -> (Signal<Option<Token>>, WriteSignal<Option<Token>>) {
    use_cookie_with_options::<_, JsonCodec>(
        "token",
        UseCookieOptions::default().max_age(60 * 60 * 1000),
    )
}

pub trait WithToken: Sized {
    /// this will try to add token to request.
    ///
    /// Will return error if token is not exist
    fn try_with_token(
        self,
        token: impl Into<MaybeSignal<Option<Token>>>,
    ) -> Result<Request<Self>>;

    /// this will try to add token to request.
    ///
    /// If token is not exist, it will just ignore error and return request without token
    fn with_token(
        self,
        token: impl Into<MaybeSignal<Option<Token>>>,
    ) -> Request<Self>;
}

impl<T> WithToken for T
where
    T: IntoRequest<T>,
{
    fn try_with_token(
        self,
        token: impl Into<MaybeSignal<Option<Token>>>,
    ) -> Result<Request<Self>> {
        let mut req = self.into_request();
        let token: MaybeSignal<_> = token.into();
        if let Some(token) = token() {
            req.metadata_mut().insert(
                "token",
                token.token.parse().map_err(|_| {
                    ErrorKind::ServerError(ServerErrorKind::InvalidValue)
                })?,
            );
        }
        Ok(req)
    }

    fn with_token(
        self,
        token: impl Into<MaybeSignal<Option<Token>>>,
    ) -> Request<Self> {
        let mut req = self.into_request();
        let token: MaybeSignal<_> = token.into();
        let Some(token) = token() else {
            return req;
        };
        let Ok(token) = token.token.parse() else {
            return req;
        };
        req.metadata_mut().insert("token", token);
        req
    }
}
