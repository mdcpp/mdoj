use std::{path::Path, sync::Arc};

use rustix::path::Arg;
use tokio::fs::{read_dir, File};

use crate::{
    filesystem::*,
    sandbox::{Cpu, Memory},
};

use super::{
    builder::*,
    spec::Spec,
    stage::{AssertionMode, Compiler, StatusCode},
};
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

impl JudgeResult {
    fn new(status: StatusCode) -> Self {
        Self {
            status,
            time: 0,
            memory: 0,
        }
    }
}

pub struct Plugin {
    pub(super) spec: Arc<Spec>,
    pub(super) template: Template<File>,
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
    pub async fn as_compiler(self: Arc<Self>, source: Vec<u8>) -> Result<Compiler> {
        let filesystem = self.template.as_filesystem(self.spec.fs_limit);
        filesystem.insert_by_path(self.spec.file.as_os_str(), source);
        Ok(Compiler::new(self.spec.clone(), filesystem.mount().await?))
    }
    pub async fn judge(self: Arc<Self>, args: JudgeArgs) -> Result<JudgeResult> {
        // for judge: it has three stages: compile, run, judge
        let compiler = self.as_compiler(args.source).await?;
        Ok(match compiler.compile().await? {
            Some(runner) => {
                let judger = runner.run((args.mem, args.cpu), args.input).await?;
                let status = judger.get_result(&args.output, args.mode);

                let stat = judger.stat();

                JudgeResult {
                    status,
                    time: stat.cpu.total,
                    memory: stat.memory.total,
                }
            }
            None => JudgeResult::new(StatusCode::CompileError),
        })
    }
    pub async fn execute(self: Arc<Self>, args: ExecuteArgs) -> Result<Option<ExecuteResult>> {
        let compiler = self.as_compiler(args.source).await?;
        Ok(match compiler.compile().await? {
            Some(runner) => {
                let judger = runner.run((args.mem, args.cpu), args.input).await?;

                todo!("stream output");

                let stat = judger.stat();

                Some(todo!())
            }
            None => None,
        })
    }
}
