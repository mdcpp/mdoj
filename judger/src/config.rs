use serde::{Deserialize, Serialize};

#[cfg(not(test))]
use std::path::PathBuf;

#[cfg(not(test))]
fn try_load_config() -> Result<Config, Box<dyn std::error::Error>> {
    use std::ops::Deref;
    use std::{fs::File, io::Read};

    let mut file = File::open("config.toml")?;
    let mut buf = String::new();
    file.read_to_string(&mut buf)?;
    let config = toml::from_str(buf.as_str())?;
    log::info!("load config from {}", CONFIG_PATH.deref().to_string_lossy());
    Ok(config)
}

#[cfg(not(test))]
lazy_static::lazy_static! {
    pub static ref CONFIG_PATH: PathBuf = PathBuf::from("config.toml");
    pub static ref CONFIG: Config=try_load_config().unwrap_or_default();
}

#[cfg(test)]
lazy_static::lazy_static! {
    pub static ref CONFIG: Config=Config::default();
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub enum Accounting {
    #[default]
    Auto,
    CpuAccounting,
    Cpu,
}

fn default_ratio_cpu() -> f32 {
    1.0
}
fn default_ratio_memory() -> f32 {
    1.0
}

#[derive(Serialize, Deserialize, Default)]
pub struct Ratio {
    #[serde(default = "default_ratio_cpu")]
    pub cpu: f32,
    #[serde(default = "default_ratio_memory")]
    pub memory: f32,
}

fn default_log() -> u8 {
    1
}

fn default_ratio() -> Ratio {
    Ratio {
        cpu: 1.0,
        memory: 1024.0,
    }
}

fn default_memory() -> u64 {
    1024 * 1024 * 1024
}

#[derive(Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub accounting: Accounting,
    #[serde(default = "default_ratio")]
    pub ratio: Ratio,
    #[serde(default)]
    pub rootless: bool,
    #[serde(default = "default_log")]
    pub log: u8,
    #[serde(default)]
    pub secret: Option<String>,
    #[serde(default = "default_memory")]
    pub memory: u64,
}
