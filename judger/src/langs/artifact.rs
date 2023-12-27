use std::{collections::BTreeMap, path::Path};

use tokio::fs;
use uuid::Uuid;

use crate::grpc::prelude::*;
use crate::init::config::CONFIG;
use crate::sandbox::prelude::*;

use super::InitError;
use super::{spec::LangSpec, Error};

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
                &spec
                    .compile_args
                    .iter()
                    .map(|x| x.as_str())
                    .collect::<Vec<_>>(),
                spec.compile_limit.clone().apply_platform(),
            )
            .await?;

        process.write_all(code).await?;

        let process = process.wait().await?;

        // if !process.succeed() {
        //     #[cfg(debug_assertions)]
        //     log::warn!("{}", process.status);
        //     return Ok(CompiledArtifact{
        //         process,
        //         spec,
        //         container:None
        //     });
        // }

        Ok(CompiledArtifact {
            process,
            spec,
            container,
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

/// Log generate from language plugin
pub struct CompileLog {
    pub level: usize,
    pub message: String,
}

impl CompileLog {
    /// parse log from raw string, slient error(generate blank message) when malformatted
    ///
    /// according to plugin specification, log should be in following format
    ///
    /// ```text
    /// 0:trace message
    /// 1:debug message
    /// 2:info message
    /// 3:warn message
    /// 4:error message
    /// ````
    pub fn from_raw(raw: &[u8]) -> Self {
        let raw: Vec<&[u8]> = raw.splitn(2, |x| *x == b':').collect();
        Self {
            level: String::from_utf8_lossy(raw[0]).parse().unwrap_or(4),
            message: String::from_utf8_lossy(raw[1]).to_string(),
        }
    }
    /// log it to the console
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

/// Wrapper for container which contain compiled program in its volume
///
/// TODO: CompiledInner<'a> was actually derive from ExitProc, consider remove CompiledInner<'a>
/// and replace it with ExitProc
pub struct CompiledArtifact<'a> {
    process: ExitProc,
    container: Container<'a>,
    spec: &'a LangSpec,
}

impl<'a> CompiledArtifact<'a> {
    /// get JudgerCode if the task is surely at state neither AC or WA
    pub fn get_expection(&self) -> Option<JudgerCode> {
        if !self.process.succeed() {
            Some(JudgerCode::Ce)
        } else {
            None
        }
    }
    pub fn log(&'a self) -> Box<dyn Iterator<Item = CompileLog> + 'a + Send> {
        Box::new(
            self.process
                .stdout
                .split(|x| *x == b'\n')
                .filter_map(|x| match x.is_empty() {
                    true => None,
                    false => Some(CompileLog::from_raw(x)),
                }),
        )
    }
}

impl<'a> CompiledArtifact<'a> {
    // run compiled program with input and limit
    pub async fn judge(
        &mut self,
        input: &[u8],
        time: u64,
        memory: u64,
    ) -> Result<TaskResult, Error> {
        debug_assert!(self.process.succeed());
        let spec = self.spec;
        let mut limit = spec.judge_limit.clone().apply_platform();

        limit.cpu_us *= time;
        limit.user_mem *= memory;

        let mut process = self
            .container
            .execute(
                &spec
                    .judge_args
                    .iter()
                    .map(|x| x.as_str())
                    .collect::<Vec<_>>(),
                limit,
            )
            .await?;

        process.write_all(input).await.ok();

        let process = process.wait().await?;

        // TODO: We should handle SysError here
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
        debug_assert!(self.process.succeed());
        let spec = self.spec;
        let mut limit = spec.judge_limit.clone().apply_platform();

        limit.cpu_us *= time;
        limit.user_mem *= memory;

        let mut process = self
            .container
            .execute(
                &spec
                    .judge_args
                    .iter()
                    .map(|x| x.as_str())
                    .collect::<Vec<_>>(),
                limit,
            )
            .await?;

        process.write_all(input).await?;

        let process = process.wait().await?;

        Ok(ExecResult(process))
    }
}

/// Wrapper for result of process(ended exec process)
///
/// provide information about process's exitcode, stdout, stderr
pub struct ExecResult(ExitProc);

impl ExecResult {
    pub fn time(&self) -> &CpuStatistics {
        &self.0.cpu
    }
    pub fn mem(&self) -> &MemStatistics {
        &self.0.mem
    }
    pub fn stdout(&self) -> &[u8] {
        &self.0.stdout
    }
}
/// Wrapper for result of process(ended judge process)
///
/// provide abliity to report resource usage, exit status, AC or WA
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
    pub fn process(&self) -> Option<&ExitProc> {
        match self {
            TaskResult::Fail(_) => None,
            TaskResult::Success(x) => Some(x),
        }
    }
    /// get JudgerCode if the task is surely at state neither AC or WA
    pub fn get_expection(&mut self) -> Option<JudgerCode> {
        match self {
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
            },
        }
    }
    // determine whether the output(stdout) match
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
    /// get cpu statistics
    pub fn cpu(&self) -> &CpuStatistics {
        &self.process().unwrap().cpu
    }
    /// get memory statistics
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
