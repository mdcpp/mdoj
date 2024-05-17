use core::time;
use std::{path::PathBuf, sync::Arc, time::Duration};

use crate::{
    filesystem::MountHandle,
    language::spec::Spec,
    sandbox::{Context, Limit, Process},
    Result,
};

use super::Runner;

pub struct Compiler {
    spec: Arc<Spec>,
    handle: MountHandle,
}

impl Compiler {
    pub fn new(spec: Arc<Spec>, handle: MountHandle) -> Self {
        Self { spec, handle }
    }
    pub async fn compile(self) -> Result<Option<Runner>> {
        let ctx = CompileCtx {
            spec: self.spec.clone(),
            path: self.handle.get_path().to_path_buf(),
        };
        let process = Process::new(ctx)?;
        let corpse = process.wait(Vec::new()).await?;
        if !corpse.success() {
            log::trace!("compile failed, corpse: {:?}", corpse);
            tokio::time::sleep(Duration::from_secs(3600)).await;
            return Ok(None);
        }

        let runner = Runner::new(self.handle, self.spec);
        Ok(Some(runner))
    }
}

struct CompileCtx {
    spec: Arc<Spec>,
    path: PathBuf,
}

impl Limit for CompileCtx {
    fn get_cpu(&mut self) -> crate::sandbox::Cpu {
        self.spec.compile_limit.cpu.clone()
    }
    fn get_memory(&mut self) -> crate::sandbox::Memory {
        self.spec.compile_limit.memory.clone()
    }
    fn get_output(&mut self) -> u64 {
        self.spec.compile_limit.output
    }
    fn get_walltime(&mut self) -> Duration {
        self.spec.compile_limit.walltime
    }
}

impl Context for CompileCtx {
    type FS = PathBuf;
    fn get_fs(&mut self) -> Self::FS {
        self.path.clone()
    }
    fn get_args(&mut self) -> impl Iterator<Item = &std::ffi::OsStr> {
        self.spec.compile_command.iter().map(|arg| arg.as_os_str())
    }
}
