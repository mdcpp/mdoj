use std::{
    ffi::OsStr,
    marker::PhantomData,
    path::{Path, PathBuf},
    sync::Arc,
};

use rustix::path::Arg;
use tokio::fs::{read_dir, File};

use crate::{
    filesystem::{Filesystem, Template},
    sandbox::{Context as SandboxCtx, Filesystem as SandboxFS, Limit},
};

use super::config::Spec;

static EXTENSION: &str = "lang";

pub async fn load_plugins(path: impl AsRef<Path>) -> std::io::Result<Vec<Plugin>> {
    let mut plugins = Vec::new();
    let mut dir_list = read_dir(path).await?;
    while let Some(entry) = dir_list.next_entry().await? {
        let path = entry.path();
        let ext = path.extension();
        if path.is_file() && ext.is_some() && ext.unwrap() == EXTENSION {
            let plugin = Plugin::new(path).await?;
            plugins.push(plugin);
        }
    }
    Ok(plugins)
}

pub struct Plugin {
    spec: Spec,
    template: Template<File>,
}

impl Plugin {
    pub async fn new(path: impl AsRef<Path> + Clone) -> std::io::Result<Self> {
        let template = Template::new(path.clone()).await?;
        let spec_source = template.read_by_path("spec.toml").await.expect(&format!(
            "sepc.toml not found in plugin {}",
            path.as_ref().display()
        ));
        let spec = Spec::from_str(&spec_source.to_string_lossy());

        Ok(Self { spec, template })
    }
    pub async fn as_runner(self: Arc<Self>) -> PluginRunner<Compile> {
        PluginRunner {
            source: self.clone(),
            filesystem: self.template.as_filesystem(0),
            _stage: PhantomData,
        }
    }
}

pub struct Compile;
pub struct Execute;

pub struct PluginRunner<S> {
    source: Arc<Plugin>,
    filesystem: Filesystem<File>,
    _stage: PhantomData<S>,
}

impl SandboxCtx for PluginRunner<Compile> {
    type FS = PathBuf;

    fn create_fs(&mut self) -> Self::FS {
        todo!()
    }

    fn get_args(&mut self) -> impl Iterator<Item = &OsStr> {
        self.source
            .spec
            .compile_command
            .iter()
            .map(|arg| arg.as_ref())
    }
}

impl Limit for PluginRunner<Compile> {
    fn get_cpu(&mut self) -> crate::sandbox::Cpu {
        todo!()
    }

    fn get_memory(&mut self) -> crate::sandbox::Memory {
        todo!()
    }

    fn get_output(&mut self) -> u64 {
        todo!()
    }
}
