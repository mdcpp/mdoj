use std::{path::PathBuf, sync::Arc, time::Duration};

use crate::{
    filesystem::MountHandle,
    language::spec::Spec,
    sandbox::{Context, Cpu, Limit, Memory, Process, Stat},
    Result,
};

use super::judge::Judger;

pub struct Runner {
    filesystem: MountHandle,
    spec: Arc<Spec>,
}

impl Runner {
    pub fn new(filesystem: MountHandle, spec: Arc<Spec>) -> Self {
        Self { filesystem, spec }
    }
    pub async fn run(self, (mem, cpu): (u64, u64), input: Vec<u8>) -> Result<Judger> {
        let ctx = RunCtx {
            spec: self.spec.clone(),
            path: self.filesystem.get_path().to_path_buf(),
            limit: self.spec.get_judge_limit(cpu, mem),
        };
        let process = Process::new(ctx)?;
        let corpse = process.wait(input).await?;
        drop(self.filesystem);
        Ok(Judger::new(self.spec, corpse))
    }
}

struct RunCtx {
    spec: Arc<Spec>,
    path: std::path::PathBuf,
    limit: Stat,
}

impl Limit for RunCtx {
    fn get_cpu(&mut self) -> Cpu {
        self.limit.cpu.clone()
    }
    fn get_memory(&mut self) -> Memory {
        self.limit.memory.clone()
    }
    fn get_output(&mut self) -> u64 {
        self.limit.output
    }
    fn get_walltime(&mut self) -> Duration {
        self.limit.walltime
    }
}

impl Context for RunCtx {
    type FS = PathBuf;
    fn get_fs(&mut self) -> Self::FS {
        self.path.clone()
    }
    fn get_args(&mut self) -> impl Iterator<Item = &std::ffi::OsStr> {
        self.spec.judge_command.iter().map(|s| s.as_ref())
    }
}
