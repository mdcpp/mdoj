use std::{collections::BTreeMap, path::Path};

use tokio::fs;

use crate::grpc::proto::prelude::JudgeResultState;
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
                self.load_module(&dir.path()).await.ok();
            }
        }
    }
    // spec would like plugins/lua-5.2/spec.toml
    // TODO: add format check
    pub async fn load_module(&mut self, spec: impl AsRef<Path>) -> Result<(), InternalError> {
        let spec = LangSpec::from_file(spec).await?;

        assert!(self.langs.insert(spec.uuid.clone(), spec).is_none());

        Ok(())
    }

    pub async fn compile(&self, uid: &UID) -> Result<CompiledArtifact, Error> {
        let spec = self.langs.get(uid).ok_or(RequestError::LangNotFound)?;

        let container = self.runtime.create(&spec.path).await.unwrap();

        let process = container
            .execute(&spec.compile_args, &spec.compile_limit)
            .await?;

        let process = process.wait().await?;

        if !process.succeed() {
            return Err(Error::Report(JudgeResultState::Ce));
        }

        Ok(CompiledArtifact { container })
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
}

pub struct TaskResult {}
