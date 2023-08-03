use std::{collections::BTreeMap, path::Path};

use tokio::fs;

use crate::grpc::proto::prelude::*;
use crate::jail::prelude::*;
use crate::{init::config::CONFIG, langs::RequestError};

use super::{spec::LangSpec, Error, InternalError};

pub type UID = String;

pub struct ArtifactFactory {
    runtime: ContainerDaemon,
    langs: BTreeMap<UID, LangSpec>,
}

impl ArtifactFactory {
    // path would like plugins/
    // TODO: add pal
    pub async fn load_dir(&mut self, path: impl AsRef<Path>) {
        let mut rd_dir = fs::read_dir(path).await.unwrap();
        while let Some(dir) = rd_dir.next_entry().await.unwrap() {
            let meta = dir.metadata().await.unwrap();
            if meta.is_dir() {
                if let Err(err)=self.load_module(&dir.path()).await{
                    log::error!("Error loading module from {}, {}",dir.path().to_string_lossy(),err);
                };
            }
        }
    }
    // spec would like plugins/lua-5.2/spec.toml
    // TODO: add format check
    pub async fn load_module(&mut self, spec: impl AsRef<Path>) -> Result<(), InternalError> {
        let spec = LangSpec::from_file(spec).await?;

        assert!(self.langs.insert(spec.uid.clone(), spec).is_none());

        Ok(())
    }

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

    pub async fn compile(&self, uid: &UID, code: &Vec<u8>) -> Result<CompiledArtifact, Error> {
        let spec = self.langs.get(uid).ok_or(RequestError::LangNotFound)?;

        let container = self.runtime.create(&spec.path).await.unwrap();

        let mut process = container
            .execute(
                &spec.compile_args,
                spec.compile_limit.clone().apply_platform(),
            )
            .await?;

        process.write_all(&code).await?;

        let process = process.wait().await?;

        if !process.succeed() {
            return Err(Error::Report(JudgeResultState::Ce));
        }

        Ok(CompiledArtifact { container, spec })
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

pub struct CompiledArtifact<'a> {
    container: Container<'a>,
    spec: &'a LangSpec,
}

impl<'a> CompiledArtifact<'a> {
    pub async fn judge(
        &mut self,
        input: &Vec<u8>,
        time: u64,
        memory: i64,
    ) -> Result<TaskResult, Error> {
        let mut limit = self.spec.judge_limit.clone().apply_platform();

        limit.cpu_us *= time;
        limit.user_mem *= memory;

        let mut process = self.container.execute(&self.spec.judge_args, limit).await?;

        process.write_all(&input).await?;

        let process = process.wait().await?;

        if !process.succeed() {
            return Err(Error::Report(JudgeResultState::Re));
        }

        Ok(TaskResult { process })
    }
}

pub struct TaskResult {
    process: ExitProc,
}

impl TaskResult {
    pub fn assert(&self, input: &Vec<u8>, mode: JudgeMatchRule) -> bool {
        let stdout = &self.process.stdout;
        // match mode{
        //     JudgeMatchRule::IgnoreSpace=>{

        //     },
        //     JudgeMatchRule::SkipSnl=>{

        //     },
        //     JudgeMatchRule::ExactSame=>{}
        // };
        todo!()
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
        self.rt_us = ((self.rt_us as f64) * config.platform.cpu_time_multiplier) as i64;
        self.total_us = ((self.total_us as f64) * config.platform.cpu_time_multiplier) as u64;

        self
    }
}