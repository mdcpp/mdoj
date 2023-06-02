use std::{collections::BTreeMap, path::Path};

use super::Error;
use serde::{Deserialize, Serialize};
use tokio::{fs, io::AsyncReadExt};

use crate::{init::config::CONFIG, jail::jail::Limit};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct PluginSpec {
    pub description: String,
    pub extension: String,
    pub uuid: String,
    pub name: String,
    pub compile: Compile,
    pub execute: Execute,
}

impl PluginSpec {
    // todo!(): perform spec check
    pub async fn from_file(path: impl AsRef<Path>) -> Result<PluginSpec, Error> {
        let mut buf = Vec::new();

        log::debug!("Loading Plugin from {}", path.as_ref().to_string_lossy());

        let mut spec = fs::File::open(path.as_ref().join("spec.toml")).await?;
        spec.read_to_end(&mut buf).await?;

        let spec = std::str::from_utf8(&buf).expect("invaild spec");

        let spec: PluginSpec = toml::from_str(spec)?;

        log::info!("Plugin {} loaded", spec.name);

        Ok(spec)
    }
    pub async fn from_root(path: impl AsRef<Path>) -> Result<BTreeMap<String, PluginSpec>, Error> {
        let mut btree = BTreeMap::new();

        let mut paths = fs::read_dir(path).await?;

        while let Some(path) = paths.next_entry().await? {
            if path.metadata().await?.is_dir()&&path.path().join("spec.toml").exists() {
                let spec = PluginSpec::from_file(path.path()).await?;
                btree.insert(spec.uuid.clone(), spec);
            }
        }

        Ok(btree)
    }
    pub fn root(&self) -> impl AsRef<Path> {
        let config = CONFIG.get().unwrap();
        Path::new(&config.plugin.path).join(&self.name)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Compile {
    pub command: Vec<String>,
    pub kernel_mem: i64,
    pub user_mem: i64,
    pub rt_time: i64,
    pub cpu_time: u64,
}

impl Compile {
    pub fn args(&self) -> Vec<&str> {
        self.command.iter().map(|s| &**s).collect()
    }
    pub fn limit(&self) -> Limit {
        Limit {
            lockdown: false,
            cpu_us: self.cpu_time,
            rt_us: self.rt_time,
            total_us: u64::MAX / 2 - 1,
            user_mem: self.user_mem,
            kernel_mem: self.kernel_mem,
            swap_user: 0,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Execute {
    pub command: Vec<String>,
    pub kernel_mem: i64,
    pub multiplier_memory: i64,
    pub rt_time: i64,
    pub multiplier_cpu: u64,
}

impl Execute {
    pub fn args(&self) -> Vec<&str> {
        self.command.iter().map(|s| &**s).collect()
    }
    pub fn limit(&self, cpu_us: u64, mem: i64) -> Limit {
        Limit {
            lockdown: false,
            cpu_us: self.multiplier_cpu * cpu_us,
            rt_us: self.rt_time,
            total_us: u64::MAX / 2 - 1,
            user_mem: self.multiplier_memory * mem,
            kernel_mem: self.kernel_mem,
            swap_user: 0,
        }
    }
}
