use std::{path::PathBuf, str::FromStr};

use serde::{Deserialize, Serialize};
use tokio::{fs, io::AsyncReadExt, sync::OnceCell};

pub static CONFIG: OnceCell<GlobalConfig> = OnceCell::const_new();

static CONFIG_PATH: &str = "config/config.toml";
static CONFIG_DIR: &str = "config";

// config
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
    #[serde(default)]
    pub secret: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Platform {
    pub cpu_time_multiplier: f64,
    pub available_memory: u64,
    pub output_limit: usize,
}

impl Default for Platform {
    fn default() -> Self {
        Self {
            cpu_time_multiplier: 1.0,
            available_memory: 1073741824,
            output_limit: 32 * 1024 * 1024,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Nsjail {
    pub runtime: String,
    pub rootless: bool,
    pub log: String,
    pub cgroup_version: CgVersion,
}

impl Nsjail {
    pub fn is_cgv1(&self) -> bool {
        self.cgroup_version == CgVersion::V1
    }
}

impl Default for Nsjail {
    fn default() -> Self {
        Self {
            runtime: "./nsjail-3.1".to_owned(),
            rootless: false,
            log: "/dev/null".to_owned(),
            cgroup_version: CgVersion::V2,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CgVersion {
    V1,
    V2,
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
            temp: PathBuf::from_str(".temp").unwrap(),
            bind: "0.0.0.0:8080".to_owned(),
            accuracy: 50 * 1000,
            root_cgroup: "mdoj/c.".to_owned(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Kernel {
    #[serde(alias = "USER_HZ")]
    pub kernel_hz: i32,
    pub tickless: bool,
}

impl Default for Kernel {
    fn default() -> Self {
        Self {
            kernel_hz: 1000,
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
                    
                    fs::create_dir_all(CONFIG_DIR).await.unwrap();

                    let config: GlobalConfig = toml::from_str("").unwrap();

                    let config_txt = toml::to_string(&config).unwrap();
                    fs::write(CONFIG_PATH, config_txt).await.unwrap();

                    println!("Finished, exiting...");
                    std::process::exit(0);
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
