use std::{path::PathBuf, sync::Arc, time::Duration};

use crate::{
    filesystem::MountHandle,
    language::spec::Spec,
    sandbox::{Context, Limit, Process},
    Result,
};

use super::JudgeRunner;

pub struct CompileRunner {
    spec: Arc<Spec>,
    handle: MountHandle,
}

impl CompileRunner {
    pub fn new(spec: Arc<Spec>, handle: MountHandle) -> Self {
        Self { spec, handle }
    }
    pub async fn run(self) -> Result<Option<JudgeRunner>> {
        let ctx = CompileCtx {
            spec: self.spec.clone(),
            path: self.handle.get_path().to_path_buf(),
        };
        let process = Process::new(ctx)?;
        let corpse = process.wait(Vec::new()).await?;
        if !corpse.success() {
            log::debug!("compile failed {:?}", corpse.status());
            return Ok(None);
        }

        let runner = JudgeRunner::new(self.handle, self.spec);
        Ok(Some(runner))
    }
}

struct CompileCtx {
    spec: Arc<Spec>,
    path: PathBuf,
}

impl Limit for CompileCtx {
    fn get_cpu(&mut self) -> crate::sandbox::Cpu {
        todo!()
    }
    fn get_memory(&mut self) -> crate::sandbox::Memory {
        todo!()
    }
    fn get_output(&mut self) -> u64 {
        todo!()
    }
    fn get_walltime(&mut self) -> Duration {
        todo!()
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
