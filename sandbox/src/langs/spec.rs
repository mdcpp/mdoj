use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tokio::{fs, io::AsyncReadExt};

use crate::{grpc::proto, init::config::CONFIG, jail::Limit};

use super::InternalError;

pub struct LangSpec {
    pub description: String,
    pub extension: String,
    pub uuid: String, // TODO
    pub name: String,
    pub compile_args: Vec<String>,
    pub compile_limit: Limit,
    pub execute_args: Vec<String>,
    pub execute_limit: Limit,
    pub path: PathBuf,
}

impl Into<proto::prelude::LangInfo> for LangSpec {
    fn into(self) -> proto::prelude::LangInfo {
        todo!()
    }
}

impl LangSpec {
    pub async fn from_file(path: impl AsRef<Path>) -> Result<Self, InternalError> {
        log::debug!("Loading Plugin from {}", path.as_ref().to_string_lossy());

        let mut buf = Vec::new();
        let mut spec = fs::File::open(path.as_ref())
            .await
            .map_err(|_| InternalError::FileNotExist)?;
        spec.read_to_end(&mut buf).await.unwrap();

        let spec = std::str::from_utf8(&buf).unwrap();

        let spec: RawLangSpec = toml::from_str(spec).map_err(|_| InternalError::FileMalFormat)?;

        let compile_limit = Limit {
            lockdown: spec.compile.lockdown,
            cpu_us: spec.compile.cpu_time,
            rt_us: spec.compile.rt_time,
            total_us: spec.compile.total_time,
            user_mem: spec.compile.user_mem,
            kernel_mem: spec.compile.kernel_mem,
            swap_user: 0,
        };

        let execute_limit = Limit {
            lockdown: true,
            cpu_us: spec.execute.multiplier_cpu,
            rt_us: spec.execute.rt_time,
            total_us: 3600 * 1000 * 1000 * 1000,
            user_mem: spec.execute.multiplier_memory,
            kernel_mem: spec.execute.kernel_mem,
            swap_user: 0,
        };

        Ok(Self {
            path: path.as_ref().parent().unwrap().join("rootfs").clone(),
            description: spec.description,
            extension: spec.extension,
            uuid: spec.uuid,
            name: spec.name,
            compile_args: spec.compile.command,
            compile_limit,
            execute_args: spec.execute.command,
            execute_limit,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct RawLangSpec {
    description: String,
    extension: String,
    uuid: String,
    name: String,
    compile: Compile,
    execute: Execute,
}

impl RawLangSpec {
    // todo!(): perform spec check
    // pub async fn from_file(path: impl AsRef<Path>) -> Result<RawLangSpec, Error> {
    //     let mut buf = Vec::new();

    //     log::debug!("Loading Plugin from {}", path.as_ref().to_string_lossy());

    //     let mut spec = fs::File::open(path.as_ref().join("spec.toml"))
    //         .await
    //         .map_err(|_| Error::FileNotExist)?;
    //     spec.read_to_end(&mut buf).await.unwrap();

    //     let spec = std::str::from_utf8(&buf).expect("invaild spec");

    //     let spec: RawLangSpec = toml::from_str(spec)?;

    //     log::info!("Plugin {} loaded", spec.name);

    //     Ok(spec)
    // }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Compile {
    lockdown: bool,
    pub command: Vec<String>,
    pub kernel_mem: i64,
    pub user_mem: i64,
    pub rt_time: i64,
    pub cpu_time: u64,
    pub total_time: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Execute {
    pub command: Vec<String>,
    pub kernel_mem: i64,
    pub multiplier_memory: i64,
    pub rt_time: i64,
    pub multiplier_cpu: u64,
}
