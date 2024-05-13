use std::{ffi::OsString, path::Path, time::Duration};

use serde::Deserialize;
use tokio::{
    fs::read_dir,
    io::{AsyncRead, AsyncReadExt},
};
use uuid::Uuid;

use crate::sandbox::{Cpu, Memory, Stat};

async fn load_plugin(path: impl AsRef<Path>) {
    let dir_list = read_dir(path).await;
}

pub struct Spec {
    pub fs_limit: u64,
    pub compile_limit: (Cpu, Memory, u64, Duration),
    judge_limit: (Cpu, Memory, u64, Duration),
    pub compile_command: Vec<OsString>,
    pub judge_command: Vec<OsString>,
    pub file: OsString,
}

impl Spec {
    pub fn get_judge_limit(&self, cpu: u64, mem: u64) -> Stat {
        todo!()
    }
    pub fn get_raw_stat(&self, stat: &Stat) -> Stat {
        todo!()
    }
    pub fn from_str(content: &str) -> Self {
        let mut raw: Raw = toml::from_str(content).unwrap();
        raw.fill();

        Self {
            fs_limit: raw.fs_limit.unwrap(),
            compile_limit: (
                Cpu {
                    kernel: raw.compile.rt_time.unwrap(),
                    user: raw.compile.cpu_time.unwrap(),
                    total: raw.compile.time.unwrap(),
                },
                Memory {
                    kernel: raw.compile.kernel_mem.unwrap(),
                    user: raw.compile.user_mem.unwrap(),
                    total: raw.compile.memory.unwrap(),
                },
                raw.compile.output_limit.unwrap(),
                Duration::from_millis(raw.compile.walltime.unwrap()),
            ),
            judge_limit: todo!(),
            compile_command: raw.compile.command,
            judge_command: raw.judge.command,
            file: raw.file,
        }
    }
}

#[derive(Deserialize)]
struct Raw {
    fs_limit: Option<u64>,
    file: OsString,
    info: String,
    extension: String,
    name: String,
    id: Uuid,
    compile: RawCompile,
    judge: RawJudge,
}

impl Raw {
    pub fn fill(&mut self) {
        if self.fs_limit.is_none() {
            self.fs_limit = Some(67108864);
        }
        self.compile.fill();
        self.judge.fill();
    }
}

#[derive(Deserialize)]
struct RawCompile {
    command: Vec<OsString>,
    kernel_mem: Option<u64>,
    memory: Option<u64>,
    user_mem: Option<u64>,
    rt_time: Option<u64>,
    cpu_time: Option<u64>,
    time: Option<u64>,
    output_limit: Option<u64>,
    walltime: Option<u64>,
}

impl RawCompile {
    fn fill(&mut self) {
        let template = Self::default();
        macro_rules! try_fill {
            ($f:ident) => {
                if self.$f.is_none(){
                    self.$f=template.$f;
                }
            };
            ($f:ident,$($e:ident),+) => {
                try_fill!($f);
                try_fill!($($e),+);
            }
        }
        try_fill!(
            kernel_mem,
            user_mem,
            rt_time,
            cpu_time,
            time,
            output_limit,
            walltime,
            memory
        );
    }
}

impl Default for RawCompile {
    fn default() -> Self {
        Self {
            command: Vec::new(),
            kernel_mem: Some(268435456),
            memory: Some(268435456),
            user_mem: Some(8589934592),
            rt_time: Some(1000000),
            cpu_time: Some(10000000000),
            time: Some(10000000),
            output_limit: Some(33554432),
            walltime: Some(360000000),
        }
    }
}

#[derive(Deserialize)]
struct RawJudge {
    command: Vec<OsString>,
    kernel_mem: Option<u64>,
    rt_time: Option<u64>,
    memory_multiplier: Option<f64>,
    cpu_multiplier: Option<f64>,
    walltime: Option<u64>,
}

impl RawJudge {
    fn fill(&mut self) {
        let template = Self::default();
        macro_rules! try_fill {
            ($f:ident) => {
                if self.$f.is_none(){
                    self.$f=template.$f;
                }
            };
            ($f:ident,$($e:ident),+) => {
                try_fill!($f);
                try_fill!($($e),+);
            }
        }
        try_fill!(
            kernel_mem,
            rt_time,
            memory_multiplier,
            cpu_multiplier,
            walltime
        );
    }
}

impl Default for RawJudge {
    fn default() -> Self {
        Self {
            command: Vec::new(),
            kernel_mem: Some(268435456),
            rt_time: Some(10000000),
            memory_multiplier: Some(1.0),
            cpu_multiplier: Some(1.0),
            walltime: Some(360000000),
        }
    }
}
