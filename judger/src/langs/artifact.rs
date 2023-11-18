use std::sync::atomic::{AtomicUsize, Ordering};
use std::{collections::BTreeMap, path::Path};

use tokio::fs;

use crate::grpc::proto::prelude::*;
use crate::sandbox::prelude::*;
use crate::{init::config::CONFIG, langs::RequestError};

use super::{spec::LangSpec, Error, InternalError};

pub type UID = String;

const TRACING_ID: AtomicUsize = AtomicUsize::new(0);

// Artifact factory, load module from disk to compile code
// Rely on container daemon to create container
pub struct ArtifactFactory {
    runtime: ContainerDaemon,
    langs: BTreeMap<UID, LangSpec>,
}

impl ArtifactFactory {
    // load all modules from a directory
    // 
    // definition of module:
    // 1. a directory with a spec.toml file inside
    // 2. resides in `path`(default to "plugins") directory
    pub async fn load_dir(&mut self, path: impl AsRef<Path>) {
        let mut rd_dir = fs::read_dir(path).await.unwrap();
        while let Some(dir) = rd_dir.next_entry().await.unwrap() {
            let meta = dir.metadata().await.unwrap();
            if meta.is_dir() {
                if let Err(err) = self.load_module(&dir.path()).await {
                    log::error!(
                        "Error loading module from {}, {}",
                        dir.path().to_string_lossy(),
                        err
                    );
                };
            }
        }
        for (uid, module) in self.langs.iter() {
            log::info!("Module {} for {}(*.{})", uid, module.name, module.extension);
        }
    }
    // adaptor, load a module from spec.toml
    pub async fn load_module(&mut self, spec: impl AsRef<Path>) -> Result<(), InternalError> {
        let spec = LangSpec::from_file(spec).await?;

        assert!(self.langs.insert(spec.uid.clone(), spec).is_none());

        Ok(())
    }
    // list all modules
    pub fn list_module(&self) -> Vec<LangInfo> {
        self.langs
            .iter()
            .map(|(_, spec)| LangInfo {
                lang_uid: spec.uid.clone(),
                lang_name: spec.name.clone(),
                info: spec.info.clone(),
                lang_ext: spec.extension.clone(),
            })
            .collect()
    }
    // compile code with sepcfication and container deamon
    pub async fn compile(&self, uid: &UID, code: &Vec<u8>) -> Result<CompiledArtifact, Error> {
        let tracing_id = TRACING_ID.fetch_add(1, Ordering::Relaxed);
        log::trace!(
            "Compiling program with module {} -trace:{}",
            uid,
            tracing_id
        );

        let spec = self
            .langs
            .get(uid)
            .ok_or(RequestError::LangNotFound(uid.clone()))?;

        let container = self.runtime.create(&spec.path).await.unwrap();

        let mut process = container
            .execute(
                spec.compile_args
                    .iter()
                    .map(|x| x.as_str())
                    .collect::<Vec<&str>>(),
                spec.compile_limit.clone().apply_platform(),
            )
            .await?;

        process.write_all(&code).await?;

        let process = process.wait().await?;

        if !process.succeed() {
            return Err(Error::Report(JudgeResultState::Ce));
        }

        Ok(CompiledArtifact {
            container,
            spec,
            tracing_id,
        })
    }
}

impl Default for ArtifactFactory {
    fn default() -> Self {
        let config = CONFIG.get().unwrap();
        Self {
            runtime: ContainerDaemon::new(config.runtime.temp.clone()),
            langs: Default::default(),
        }
    }
}
// Wrapper for container which contain compiled program in its volume
pub struct CompiledArtifact<'a> {
    container: Container<'a>,
    spec: &'a LangSpec,
    tracing_id: usize,
}

impl<'a> CompiledArtifact<'a> {
    // run compiled program with input and limit
    pub async fn judge(
        &mut self,
        input: &Vec<u8>,
        time: u64,
        memory: u64,
    ) -> Result<TaskResult, Error> {
        log::trace!("Running program -trace:{}", self.tracing_id);
        let mut limit = self.spec.judge_limit.clone().apply_platform();

        limit.cpu_us *= time;
        limit.user_mem *= memory;

        let mut process = self
            .container
            .execute(
                self.spec
                    .judge_args
                    .iter()
                    .map(|x| x.as_str())
                    .collect::<Vec<&str>>(),
                limit,
            )
            .await?;

        process.write_all(&input).await?;

        let process = process.wait().await?;

        if !process.succeed() {
            return Err(Error::Report(JudgeResultState::Re));
        }

        Ok(TaskResult {
            process,
            tracing_id: self.tracing_id,
        })
    }
}
// Wrapper for result of process(ended process)
// provide information about process's exitcode, resource usage, stdout, stderr
pub struct TaskResult {
    process: ExitProc,
    tracing_id: usize,
}

impl TaskResult {
    pub fn get_expection(&self) -> Option<JudgeResultState> {
        match self.process.status {
            ExitStatus::SigExit => Some(JudgeResultState::Rf),
            ExitStatus::Code(code) => match code {
                0 | 5 | 6 | 9 => None,
                137 => Some(JudgeResultState::Na),
                _ => Some(JudgeResultState::Rf),
            },
            ExitStatus::MemExhausted => Some(JudgeResultState::Mle),
            ExitStatus::CpuExhausted => Some(JudgeResultState::Tle),
            ExitStatus::SysError => Some(JudgeResultState::Na),
        }
    }
    pub fn assert(&self, input: &Vec<u8>, mode: JudgeMatchRule) -> bool {
        let newline = '\n' as u8;
        let space = ' ' as u8;
        log::trace!("Ssserting program -trace:{}", self.tracing_id);
        let stdout = &self.process.stdout;

        match mode {
            JudgeMatchRule::ExactSame => stdout.iter().zip(input.iter()).all(|(f, s)| f == s),
            JudgeMatchRule::IgnoreSnl => {
                let stdout_split = stdout.split(|x| *x == newline || *x == space);
                let input_split = input.split(|x| *x == newline || *x == space);
                for (f, s) in stdout_split.zip(input_split) {
                    if f.iter().zip(s.iter()).any(|(f, s)| f != s) {
                        return false;
                    }
                }
                return true;
            }
            JudgeMatchRule::SkipSnl => {
                let stdout_filtered = stdout.iter().filter(|x| **x != newline || **x != space);
                let input_filtered = input.iter().filter(|x| **x != newline || **x != space);

                stdout_filtered.zip(input_filtered).all(|(f, s)| f == s)
            }
        }
    }
    pub fn time(&self) -> &CpuStatistics {
        &self.process.cpu
    }
    pub fn mem(&self) -> &MemStatistics {
        &self.process.mem
    }
}

impl Limit {
    fn apply_platform(mut self) -> Self {
        let config = CONFIG.get().unwrap();

        self.cpu_us = ((self.cpu_us as f64) * config.platform.cpu_time_multiplier) as u64;
        self.rt_us = ((self.rt_us as f64) * config.platform.cpu_time_multiplier) as u64;
        self.total_us = ((self.total_us as f64) * config.platform.cpu_time_multiplier) as u64;

        self
    }
}
