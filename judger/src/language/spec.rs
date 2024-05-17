use std::{ffi::OsString, str::FromStr, time::Duration};

use grpc::judger::LangInfo;
use serde::Deserialize;
use uuid::Uuid;

use crate::sandbox::{Cpu, Memory, Stat};

pub struct CpuFactor {
    kernel: u64,
    user: u64,
    total: f64,
}

impl CpuFactor {
    pub fn get(&self, cpu: u64) -> Cpu {
        Cpu {
            kernel: self.kernel,
            user: self.user,
            total: (cpu as f64 * self.total) as u64,
        }
    }
    pub fn get_raw(&self, cpu: Cpu) -> Cpu {
        Cpu {
            kernel: (cpu.kernel as f64 / self.total) as u64,
            user: (cpu.user as f64 / self.total) as u64,
            total: (cpu.total as f64 / self.total) as u64,
        }
    }
}

pub struct MemFactor {
    kernel: u64,
    user: u64,
    total: f64,
}

impl MemFactor {
    pub fn get(&self, mem: u64) -> Memory {
        Memory {
            kernel: self.kernel,
            user: self.user,
            total: (mem as f64 * self.total) as u64,
        }
    }
    pub fn get_raw(&self, mem: Memory) -> Memory {
        Memory {
            kernel: ((mem.kernel as f64) / self.total) as u64,
            user: ((mem.user as f64) / self.total) as u64,
            total: ((mem.total as f64) / self.total) as u64,
        }
    }
}

pub struct Spec {
    pub id: Uuid,
    pub fs_limit: u64,
    pub compile_limit: Stat,
    judge_cpu_factor: CpuFactor,
    judge_mem_factor: MemFactor,
    judge_limit: (u64, Duration),
    pub compile_command: Vec<OsString>,
    pub judge_command: Vec<OsString>,
    pub file: OsString,
    pub info: LangInfo,
}

impl Spec {
    pub fn get_judge_limit(&self, cpu: u64, mem: u64) -> Stat {
        let cpu = self.judge_cpu_factor.get(cpu);
        let mem = self.judge_mem_factor.get(mem);
        Stat {
            cpu,
            memory: mem,
            output: self.judge_limit.0,
            walltime: self.judge_limit.1,
        }
    }
    pub fn get_raw_stat(&self, stat: &Stat) -> Stat {
        let mut stat = stat.clone();
        stat.cpu = self.judge_cpu_factor.get_raw(stat.cpu);
        stat.memory = self.judge_mem_factor.get_raw(stat.memory);
        stat
    }
    pub fn get_memory_reserved_size(&self, mem: u64) -> u64 {
        self.judge_mem_factor.get(mem).get_reserved_size() + self.fs_limit
    }
    pub fn from_str(content: &str) -> Self {
        let mut raw: Raw = toml::from_str(content).unwrap();
        raw.fill();

        // FIXME: use compsition instead
        Self {
            info: LangInfo::from(&raw),
            id: raw.id,
            fs_limit: raw.fs_limit.unwrap(),
            compile_limit: Stat {
                cpu: Cpu {
                    kernel: raw.compile.rt_time.unwrap(),
                    user: raw.compile.cpu_time.unwrap(),
                    total: raw.compile.time.unwrap(),
                },
                memory: Memory {
                    kernel: raw.compile.kernel_mem.unwrap(),
                    user: raw.compile.user_mem.unwrap(),
                    total: raw.compile.memory.unwrap(),
                },
                output: raw.compile.output_limit.unwrap(),
                walltime: Duration::from_nanos(raw.compile.walltime.unwrap()),
            },
            compile_command: raw
                .compile
                .command
                .iter()
                .map(|x| OsString::from(x))
                .collect(),
            judge_command: raw
                .judge
                .command
                .iter()
                .map(|x| OsString::from(x))
                .collect(),
            file: OsString::from(raw.file),
            judge_cpu_factor: CpuFactor {
                kernel: raw.judge.kernel_mem.unwrap(),
                user: raw.judge.rt_time.unwrap(),
                total: raw.judge.cpu_multiplier.unwrap(),
            },
            judge_mem_factor: MemFactor {
                kernel: raw.judge.kernel_mem.unwrap(),
                user: raw.judge.rt_time.unwrap(),
                total: raw.judge.memory_multiplier.unwrap(),
            },
            judge_limit: (
                raw.judge.walltime.unwrap(),
                Duration::from_nanos(raw.judge.rt_time.unwrap()),
            ),
        }
    }
}

#[derive(Deserialize)]
pub struct Raw {
    fs_limit: Option<u64>,
    file: String,
    info: String,
    extension: String,
    name: String,
    id: Uuid,
    compile: RawCompile,
    judge: RawJudge,
}

impl<'a> From<&'a Raw> for LangInfo {
    fn from(value: &'a Raw) -> Self {
        LangInfo {
            lang_uid: value.id.to_string(),
            lang_name: value.name.clone(),
            info: value.info.clone(),
            lang_ext: value.extension.clone(),
        }
    }
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
    command: Vec<String>,
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
    command: Vec<String>,
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
