use std::{ffi::OsString, time::Duration};

use grpc::judger::LangInfo;
use uuid::Uuid;

use crate::sandbox::{Cpu, Memory, Stat};

use self::raw::Raw;

mod raw;

/// multiplier for cpu limit
///
/// To make setting different time limit for different programing language
/// transparent to backend, we set different factor to different language
#[derive(Clone)]
pub struct CpuFactor {
    kernel: u64,
    user: u64,
    total: f64,
}

impl CpuFactor {
    /// create a [`Cpu`] limit given total cpu limit
    pub fn create_from(&self, cpu: u64) -> Cpu {
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

/// multiplier for memory limit
///
/// To make setting different time limit for different programing language
/// transparent to backend, we set different factor to different language
#[derive(Clone)]
pub struct MemFactor {
    kernel: u64,
    user: u64,
    total: f64,
}

impl MemFactor {
    /// create a [`Memory`] limit given total memory limit
    pub fn create_from(&self, total: u64) -> Memory {
        Memory {
            kernel: self.kernel,
            user: self.user,
            total: (total as f64 * self.total) as u64,
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
        let cpu = self.judge_cpu_factor.create_from(cpu);
        let mem = self.judge_mem_factor.create_from(mem);
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
    /// get size of memory that should be reserved for the sandbox to prevent OOM
    pub fn get_memory_reserved_size(&self, mem: u64) -> u64 {
        self.judge_mem_factor.create_from(mem).get_reserved_size() + self.fs_limit
    }
    pub fn from_str(content: &str) -> Self {
        let mut raw: Raw = toml::from_str(content).unwrap();
        raw.fill();

        // FIXME: use composition instead
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
            compile_command: raw.compile.command.iter().map(OsString::from).collect(),
            judge_command: raw.judge.command.iter().map(OsString::from).collect(),
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
                raw.judge.output.unwrap(),
                Duration::from_nanos(raw.judge.walltime.unwrap()),
            ),
        }
    }
}
