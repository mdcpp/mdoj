use std::borrow::Cow;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{collections::BTreeMap, path::Path};

use tokio::fs;
use uuid::Uuid;

use crate::grpc::proto::prelude::*;
use crate::init::config::CONFIG;
use crate::sandbox::prelude::*;

use super::InitError;
use super::{spec::LangSpec, Error};

static TRACING_ID: AtomicUsize = AtomicUsize::new(0);
// Artifact factory, load module from disk to compile code
// Rely on container daemon to create container
pub struct ArtifactFactory {
    runtime: ContainerDaemon,
    langs: BTreeMap<Uuid, LangSpec>,
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
    pub async fn load_module(&mut self, spec: impl AsRef<Path>) -> Result<(), InitError> {
        let spec = LangSpec::from_file(spec).await?;

        assert!(self.langs.insert(spec.uid, spec).is_none());

        Ok(())
    }
    // list all modules
    pub fn list_module(&self) -> Vec<LangInfo> {
        self.langs
            .values()
            .map(|spec| LangInfo {
                lang_uid: spec.uid.clone().to_string(),
                lang_name: spec.name.clone(),
                info: spec.info.clone(),
                lang_ext: spec.extension.clone(),
            })
            .collect()
    }
    // compile code with sepcfication and container deamon
    pub async fn compile(&self, uid: &Uuid, code: &[u8]) -> Result<CompiledArtifact, Error> {
        log::trace!("Compiling program with module {}", uid,);

        let spec = self.langs.get(uid).ok_or(Error::LangNotFound)?;

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

        process.write_all(code).await?;

        let process = process.wait().await?;

        if !process.succeed() {
            #[cfg(debug_assertions)]
            log::warn!("{}", process.status);
            return Ok(CompiledArtifact::Fail(JudgerCode::Ce));
        }

        process.stdout.split(|x| *x == b'\n').for_each(|x| {
            CompileLog::from_raw(x).log();
        });

        Ok(CompiledArtifact::Success(CompiledInner {
            container,
            spec,
            stdout: process.stdout,
        }))
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

pub struct CompileLog<'a> {
    pub level: usize,
    pub message: Cow<'a, str>,
}

impl<'a> CompileLog<'a> {
    pub fn from_raw(raw: &'a [u8]) -> Self {
        let raw: Vec<&[u8]> = raw.splitn(2, |x| *x == b':').collect();
        if raw.len() == 1 {
            Self {
                level: 4,
                message: String::from_utf8_lossy(raw[0]),
            }
        } else {
            Self {
                level: String::from_utf8_lossy(raw[0]).parse().unwrap_or(4),
                message: String::from_utf8_lossy(raw[1]),
            }
        }
    }
    pub fn log(&self) {
        match self.level {
            0 => log::trace!("{}", self.message),
            1 => log::debug!("{}", self.message),
            2 => log::info!("{}", self.message),
            3 => log::warn!("{}", self.message),
            4 => log::error!("{}", self.message),
            _ => {}
        }
    }
}
// Wrapper for container which contain compiled program in its volume
pub enum CompiledArtifact<'a> {
    Fail(JudgerCode),
    Success(CompiledInner<'a>),
}

impl<'a> CompiledArtifact<'a> {
    pub fn get_expection(&self) -> Option<JudgerCode> {
        match self {
            CompiledArtifact::Fail(x) => Some(*x),
            CompiledArtifact::Success(_) => None,
        }
    }
    fn inner(&mut self) -> Option<&mut CompiledInner<'a>> {
        match self {
            CompiledArtifact::Fail(x) => None,
            CompiledArtifact::Success(x) => Some(x),
        }
    }
}
pub struct CompiledInner<'a> {
    container: Container<'a>,
    spec: &'a LangSpec,
    stdout: Vec<u8>,
}

impl<'a> CompiledArtifact<'a> {
    // run compiled program with input and limit
    pub async fn judge(
        &mut self,
        input: &[u8],
        time: u64,
        memory: u64,
    ) -> Result<TaskResult, Error> {
        let inner = self.inner().unwrap();
        let mut limit = inner.spec.judge_limit.clone().apply_platform();

        limit.cpu_us *= time;
        limit.user_mem *= memory;

        let mut process = inner
            .container
            .execute(
                inner
                    .spec
                    .judge_args
                    .iter()
                    .map(|x| x.as_str())
                    .collect::<Vec<&str>>(),
                limit,
            )
            .await?;

        process.write_all(input).await.ok();

        let process = process.wait().await?;

        if !process.succeed() {
            // log::debug!("process status: {:?}", process.status);
            return Ok(TaskResult::Fail(JudgerCode::Re));
        }

        Ok(TaskResult::Success(process))
    }
    pub async fn exec(
        &mut self,
        input: &[u8],
        time: u64,
        memory: u64,
    ) -> Result<ExecResult, Error> {
        let inner = self.inner().unwrap();
        let mut limit = inner.spec.judge_limit.clone().apply_platform();

        limit.cpu_us *= time;
        limit.user_mem *= memory;

        let mut process = inner
            .container
            .execute(
                inner
                    .spec
                    .judge_args
                    .iter()
                    .map(|x| x.as_str())
                    .collect::<Vec<&str>>(),
                limit,
            )
            .await?;

        process.write_all(input).await?;

        let process = process.wait().await?;

        Ok(ExecResult { process })
    }

    pub fn to_log(&mut self) -> impl Iterator<Item = CompileLog> {
        let inner = self.inner().unwrap();
        inner
            .stdout
            .split(|&x| x == b'\n')
            .map(CompileLog::from_raw)
    }
}

pub struct ExecResult {
    process: ExitProc,
}

impl ExecResult {
    pub fn time(&self) -> &CpuStatistics {
        &self.process.cpu
    }
    pub fn mem(&self) -> &MemStatistics {
        &self.process.mem
    }
    pub fn stdout(&self) -> &[u8] {
        &self.process.stdout
    }
}
// Wrapper for result of process(ended judge process)
// provide information about process's exitcode, resource usage, stdout, stderr
pub enum TaskResult {
    Fail(JudgerCode),
    Success(ExitProc),
}

impl TaskResult {
    fn process_mut(&mut self) -> Option<&mut ExitProc> {
        match self {
            TaskResult::Fail(_) => None,
            TaskResult::Success(x) => Some(x),
        }
    }
}

impl TaskResult {
    fn process(&self) -> Option<&ExitProc> {
        match self {
            TaskResult::Fail(x) => None,
            TaskResult::Success(x) => Some(x),
        }
    }
}
impl TaskResult {
    pub fn get_expection(&mut self) -> Option<JudgerCode> {
        match self{
            TaskResult::Fail(x) => Some(*x),
            TaskResult::Success(process) => match process.status {
                ExitStatus::SigExit(sig) => match sig {
                    11 => Some(JudgerCode::Re),
                    _ => Some(JudgerCode::Rf),
                },
                ExitStatus::Code(code) => match code {
                    125 => Some(JudgerCode::Mle),
                    126 | 127 | 129..=192 => Some(JudgerCode::Rf),
                    255 | 0..=124 => None,
                    _ => Some(JudgerCode::Na),
                },
                ExitStatus::MemExhausted => Some(JudgerCode::Mle),
                ExitStatus::CpuExhausted => Some(JudgerCode::Tle),
                ExitStatus::SysError => Some(JudgerCode::Na),
            }
        }
    }
    pub fn assert(&mut self, input: &[u8], mode: JudgeMatchRule) -> bool {
        let newline = b'\n';
        let space = b' ';
        let stdout = &self.process_mut().unwrap().stdout;

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
                true
            }
            JudgeMatchRule::SkipSnl => {
                let stdout_filtered = stdout.iter().filter(|x| **x != newline || **x != space);
                let input_filtered = input.iter().filter(|x| **x != newline || **x != space);

                stdout_filtered.zip(input_filtered).all(|(f, s)| f == s)
            }
        }
    }
    pub fn time(&self) -> &CpuStatistics {
        &self.process().unwrap().cpu
    }
    pub fn mem(&self) -> &MemStatistics {
        &self.process().unwrap().mem
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
