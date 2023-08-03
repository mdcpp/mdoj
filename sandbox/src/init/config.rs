use std::{path::PathBuf, str::FromStr};

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
    pub platform: Platform,
    #[serde(default)]
    pub nsjail: Nsjail,
    #[serde(default)]
    pub plugin: Plugin,
    #[serde(default)]
    pub kernel: Kernel,
    #[serde(default)]
    pub log_level: usize,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Platform {
    pub cpu_time_multiplier: f64,
    pub available_memory: i64,
}

impl Default for Platform {
    fn default() -> Self {
        Self {
            cpu_time_multiplier: 1.0,
            available_memory: 1073741824,
        }
    }
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
    pub temp: PathBuf,
    pub bind: String,
    pub accuracy: u64,
    pub root_cgroup: String,
}

impl Default for Runtime {
    fn default() -> Self {
        Self {
            temp: PathBuf::from_str("temp").unwrap(),
            bind: "0.0.0.0:8080".to_owned(),
            accuracy: 50 * 1000,
            root_cgroup: "mdoj".to_owned(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Kernel {
    pub USER_HZ: i32,
    pub tickless: bool,
}

impl Default for Kernel {
    fn default() -> Self {
        Self {
            USER_HZ: 100,
            tickless: false,
        }
    }
}

pub async fn init() {
    let mut buf = Vec::new();

    let config_file = fs::File::open(CONFIG_PATH).await;

    match CONFIG.get() {
        Some(_) => {
            #[cfg(not(test))]
            panic!("config have been set twice, which indicated a bug in the program");
        }
        None => {
            let config: GlobalConfig = match config_file {
                Ok(mut x) => {
                    if x.metadata().await.unwrap().is_file() {
                        x.read_to_end(&mut buf).await.unwrap();
                        let config = std::str::from_utf8(&buf)
                            .expect("Unable to parse config, Check config is correct");
                        toml::from_str(config).unwrap()
                    } else {
                        panic!(
                            "Unable to open config file, {} should not be symlink or folder",
                            CONFIG_PATH
                        );
                    }
                }
                Err(_) => {
                    println!("Unable to find {}, generating default config", CONFIG_PATH);

                    let config: GlobalConfig = toml::from_str("").unwrap();

                    let config_txt = toml::to_string(&config).unwrap();
                    fs::write(CONFIG_PATH, config_txt).await.unwrap();

                    config
                }
            };

            CONFIG.set(config).ok();
        }
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
