use std::{path::Path, sync::Arc};

use rustix::path::Arg;
use tokio::fs::{read_dir, File};

use crate::filesystem::*;

use super::{spec::Spec, stage::CompileRunner};
use crate::Result;

static EXTENSION: &str = "lang";

pub async fn load_plugins(path: impl AsRef<Path>) -> Result<Vec<Plugin>> {
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
    spec: Arc<Spec>,
    template: Template<File>,
}

impl Plugin {
    pub async fn new(path: impl AsRef<Path> + Clone) -> Result<Self> {
        let template = Template::new(path.clone()).await?;
        let spec_source = template.read_by_path("spec.toml").await.expect(&format!(
            "sepc.toml not found in plugin {}",
            path.as_ref().display()
        ));
        let spec = Arc::new(Spec::from_str(&spec_source.to_string_lossy()));

        Ok(Self { spec, template })
    }
    pub async fn as_runner(self: Arc<Self>) -> Result<CompileRunner> {
        let filesystem = self
            .template
            .as_filesystem(self.spec.fs_limit)
            .mount()
            .await?;
        Ok(CompileRunner::new(self.spec.clone(), filesystem))
    }
}
