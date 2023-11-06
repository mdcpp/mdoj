use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tokio::{fs, io::AsyncReadExt};

use crate::sandbox::Limit;

use super::InternalError;

pub struct LangSpec {
    pub info: String,
    pub extension: String,
    pub uid: String, // TODO
    pub name: String,
    pub compile_args: Vec<String>,
    pub compile_limit: Limit,
    pub judge_args: Vec<String>,
    pub judge_limit: Limit,
    pub path: PathBuf,
}

impl LangSpec {
    pub async fn from_file(path: impl AsRef<Path>) -> Result<Self, InternalError> {
        log::trace!("Loading module from {}", path.as_ref().to_string_lossy());

        let mut buf = Vec::new();
        let mut spec = fs::File::open(path.as_ref().join("spec.toml"))
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

        let judge_limit = Limit {
            lockdown: true,
            cpu_us: spec.judge.multiplier_cpu,
            rt_us: spec.judge.rt_time,
            total_us: 3600 * 1000 * 1000 * 1000,
            user_mem: spec.judge.multiplier_memory,
            kernel_mem: spec.judge.kernel_mem,
            swap_user: 0,
        };

        Ok(Self {
            path: path.as_ref().join("rootfs").clone(),
            info: spec.info,
            extension: spec.extension,
            uid: spec.uid,
            name: spec.name,
            compile_args: spec.compile.command,
            compile_limit,
            judge_args: spec.judge.command,
            judge_limit,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct RawLangSpec {
    info: String,
    extension: String,
    uid: String,
    name: String,
    compile: Compile,
    judge: Judge,
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
pub struct Judge {
    pub command: Vec<String>,
    pub kernel_mem: i64,
    pub multiplier_memory: i64,
    pub rt_time: i64,
    pub multiplier_cpu: u64,
}
