use std::{path::PathBuf, sync::Arc, time::Duration};

use crate::{
    filesystem::MountHandle,
    language::spec::Spec,
    sandbox::{Context, Cpu, Limit, Memory, Process},
    Result,
};

use super::assert::AssertRunner;

pub struct JudgeRunner {
    filesystem: MountHandle,
    spec: Arc<Spec>,
}

impl JudgeRunner {
    pub fn new(filesystem: MountHandle, spec: Arc<Spec>) -> Self {
        Self { filesystem, spec }
    }
    pub async fn run(self, (mem, cpu): (Memory, Cpu), input: Vec<u8>) -> Result<AssertRunner> {
        let ctx = JudgeCtx {
            spec: self.spec.clone(),
            path: self.filesystem.get_path().to_path_buf(),
            limit: self.spec.get_judge_limit(cpu, mem),
        };
        let process = Process::new(ctx)?;
        let corpse = process.wait(input).await?;
        drop(self.filesystem);
        Ok(AssertRunner::new(self.spec, corpse))
    }
}

struct JudgeCtx {
    spec: Arc<Spec>,
    path: std::path::PathBuf,
    limit: (Cpu, Memory, u64, Duration),
}

impl Limit for JudgeCtx {
    fn get_cpu(&mut self) -> Cpu {
        self.limit.0.clone()
    }
    fn get_memory(&mut self) -> Memory {
        self.limit.1.clone()
    }
    fn get_output(&mut self) -> u64 {
        self.limit.2
    }
    fn get_walltime(&mut self) -> Duration {
        self.limit.3
    }
}

impl Context for JudgeCtx {
    type FS = PathBuf;
    fn get_fs(&mut self) -> Self::FS {
        self.path.clone()
    }
    fn get_args(&mut self) -> impl Iterator<Item = &std::ffi::OsStr> {
        self.spec.judge_command.iter().map(|s| s.as_ref())
    }
}
