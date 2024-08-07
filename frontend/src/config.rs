use std::rc::Rc;
#[cfg(feature = "ssr")]
use std::sync::OnceLock;

use leptos::*;
use serde::{Deserialize, Serialize};

use crate::errors::*;

#[cfg(feature = "ssr")]
static CONFIG: OnceLock<Config> = OnceLock::new();

#[derive(Debug, PartialEq, Eq, Clone, Default, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub frontend: FrontendConfig,
    #[serde(default)]
    pub backend: BackendConfig,
}

#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
pub struct FrontendConfig {
    #[serde(default = "default_image_providers")]
    pub image_providers: Vec<String>,

    #[serde(default = "default_api_server")]
    pub api_server: String,

    #[serde(default = "default_extension_language_mappings")]
    pub extension_language_mappings: Vec<ExtensionLanguageMapping>,

    #[serde(default = "default_page_size")]
    pub page_size: usize,
}

#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
pub struct ExtensionLanguageMapping {
    pub extension: Vec<String>,
    pub language: String,
}

fn default_api_server() -> String {
    "http://0.0.0.0:8081".to_owned()
}

fn default_image_providers() -> Vec<String> {
    vec!["https://i.imgur.com".to_owned()]
}

fn default_extension_language_mappings() -> Vec<ExtensionLanguageMapping> {
    vec![
        ExtensionLanguageMapping {
            extension: vec!["md".to_owned()],
            language: "markdown".to_owned(),
        },
        ExtensionLanguageMapping {
            extension: vec!["js".to_owned()],
            language: "javascript".to_owned(),
        },
        ExtensionLanguageMapping {
            extension: vec!["c".to_owned()],
            language: "c".to_owned(),
        },
        ExtensionLanguageMapping {
            extension: vec![
                "c++".to_owned(),
                "cpp".to_owned(),
                "cp".to_owned(),
                "cxx".to_owned(),
            ],
            language: "javascript".to_owned(),
        },
        ExtensionLanguageMapping {
            extension: vec!["lua".to_owned()],
            language: "lua".to_owned(),
        },
    ]
}

fn default_page_size() -> usize {
    50
}

impl Default for FrontendConfig {
    fn default() -> Self {
        Self {
            image_providers: default_image_providers(),
            api_server: default_api_server(),
            extension_language_mappings: default_extension_language_mappings(),
            page_size: default_page_size(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
pub struct BackendConfig {
    #[serde(default)]
    pub trust_xff: bool,
}

impl Default for BackendConfig {
    fn default() -> Self {
        Self { trust_xff: false }
    }
}

#[cfg(feature = "ssr")]
pub async fn init_config() -> Result<()> {
    let config = load_config().await?;
    let _ = CONFIG.set(config);
    Ok(())
}

#[cfg(feature = "ssr")]
async fn load_config() -> Result<Config> {
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
        return Ok(toml::from_str(&buf).map_err(|_| Error {
            kind: ErrorKind::Internal,
            context: "malformed config file".to_owned(),
        })?);
    }
    let default_config = Config::default();
    let default_toml = toml::to_string_pretty(&default_config)
        .expect("Cannot generate default config");
    fs::write(&config_path, default_toml).await?;
    Ok(default_config)
}

#[cfg(feature = "ssr")]
pub fn frontend_config() -> &'static FrontendConfig {
    CONFIG
        .get()
        .map(|c| &c.frontend)
        .expect("config is not init!")
}

#[cfg(not(feature = "ssr"))]
pub fn frontend_config() -> Rc<FrontendConfig> {
    expect_context()
}

#[cfg(feature = "ssr")]
pub fn backend_config() -> &'static BackendConfig {
    CONFIG
        .get()
        .map(|c| &c.backend)
        .expect("config is not init!")
}

#[cfg(feature = "ssr")]
#[component]
pub fn ProvideConfig(children: Children) -> impl IntoView {
    let json =
        serde_json::to_string(frontend_config()).expect("Cannot to json");
    provide_context(Rc::new(frontend_config().to_owned()));
    view! {
        <script id="config" type="application/json">
            {json}
        </script>
        {children()}
    }
}

#[cfg(not(feature = "ssr"))]
#[component]
pub fn ProvideConfig(children: Children) -> impl IntoView {
    let json = document()
        .get_element_by_id("config")
        .unwrap()
        .text_content()
        .unwrap();

    let config: FrontendConfig = serde_json::from_str(&json).unwrap();
    provide_context(Rc::new(config));
    view! {
        <script id="config" type="application/json">
            {json}
        </script>
        {children()}
    }
}
