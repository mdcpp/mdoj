use std::{collections::BTreeMap, path::Path, pin::Pin, sync::Arc};

use async_stream::{stream, try_stream};
use futures_core::Stream;
use grpc::judger::LangInfo;
use rustix::path::Arg;
use tokio::{
    fs::{read_dir, File},
    io::{AsyncRead, AsyncSeek},
};
use uuid::Uuid;

use crate::{filesystem::*, sandbox::Stat};

use super::{
    builder::*,
    spec::Spec,
    stage::{Compiler, StatusCode},
};
use crate::Result;

macro_rules! trys {
    ($ele:expr) => {
        match $ele {
            Ok(x) => x,
            Err(err) => {
                return Box::pin(stream! {
                    yield Err(err);
                });
            }
        }
    };
    ($ele:expr,$ret:expr) => {
        match $ele {
            Some(x) => x,
            None => {
                return Box::pin(stream! {yield $ret;});
            }
        }
    };
}

static EXTENSION: &str = "lang";

pub async fn load_plugins(path: impl AsRef<Path>) -> Result<Vec<Plugin<File>>> {
    let mut plugins = Vec::new();
    let mut dir_list = read_dir(path).await?;
    while let Some(entry) = dir_list.next_entry().await? {
        let path = entry.path();
        log::trace!("find potential plugin from {}", path.display());
        let ext = path.extension();
        if path.is_file() && ext.is_some() && ext.unwrap() == EXTENSION {
            log::info!("load plugin from {}", path.display());
            let plugin = Plugin::new(path).await?;
            plugins.push(plugin);
        }
    }
    Ok(plugins)
}

pub struct Map<F>(BTreeMap<Uuid, Plugin<F>>)
where
    F: AsyncRead + AsyncSeek + Unpin + Send + 'static;

impl Map<File> {
    pub async fn new(path: impl AsRef<Path>) -> Result<Self> {
        let plugins = load_plugins(path).await?;
        let mut map = BTreeMap::new();

        for plugin in plugins {
            map.insert(plugin.spec.id, plugin);
        }
        Ok(Self(map))
    }
    pub fn get(&self, id: &Uuid) -> Option<Plugin<File>> {
        self.0.get(id).cloned()
    }
    pub fn iter(&self) -> impl Iterator<Item = &Plugin<File>> {
        self.0.values()
    }
}

impl JudgeResult {
    fn compile_error() -> Self {
        Self {
            status: StatusCode::CompileError,
            time: 0,
            memory: 0,
        }
    }
}

pub struct Plugin<F>
where
    F: AsyncRead + AsyncSeek + Unpin + Send + 'static,
{
    pub(super) spec: Arc<Spec>,
    pub(super) template: Arc<Template<F>>,
}

impl<F> Clone for Plugin<F>
where
    F: AsyncRead + AsyncSeek + Unpin + Send + 'static,
{
    fn clone(&self) -> Self {
        Self {
            spec: self.spec.clone(),
            template: self.template.clone(),
        }
    }
}

impl Plugin<File> {
    pub async fn new(path: impl AsRef<Path> + Clone) -> Result<Self> {
        let template = Arc::new(Template::new(path.clone()).await?);
        let spec_source = template.read_by_path("spec.toml").await.expect(&format!(
            "sepc.toml not found in plugin {}",
            path.as_ref().display()
        ));
        let spec = Arc::new(Spec::from_str(&spec_source.to_string_lossy()));

        Ok(Self { spec, template })
    }
}

impl<F> Plugin<F>
where
    F: AsyncRead + AsyncSeek + Unpin + Send + Sync + 'static,
{
    pub fn get_info(&self) -> &LangInfo {
        &self.spec.info
    }
    pub async fn as_compiler(&self, source: Vec<u8>) -> Result<Compiler> {
        log::trace!(
            "create compiler from plugin {}",
            self.spec.info.lang_name.as_str()
        );
        let filesystem = self.template.as_filesystem(self.spec.fs_limit);
        filesystem.insert_by_path(self.spec.file.as_os_str(), source);
        Ok(Compiler::new(self.spec.clone(), filesystem.mount().await?))
    }
    pub async fn judge(
        &self,
        args: JudgeArgs,
    ) -> Pin<Box<dyn Stream<Item = Result<JudgeResult>> + Send>> {
        let compiler = trys!(self.as_compiler(args.source).await);
        let maybe_runner = trys!(compiler.compile().await);
        log::debug!("runner created");
        let mut runner = trys!(maybe_runner, Ok(JudgeResult::compile_error()));

        let mem_cpu = (args.mem, args.cpu);
        let mode = args.mode;
        let mut io = args.input.into_iter().zip(args.output.into_iter());
        Box::pin(try_stream! {
            while let Some((input,output))=io.next(){
                let judger = runner.run(mem_cpu.clone(), input).await?;
                let status = judger.get_result(&output, mode);

                let stat = judger.stat();
                yield JudgeResult {
                    status,
                    time: stat.cpu.total,
                    memory: stat.memory.total,
                };
                if status!=StatusCode::Accepted{
                    break;
                }
            }
        })
    }
    pub async fn execute(&self, args: ExecuteArgs) -> Result<Option<ExecuteResult>> {
        let compiler = self.as_compiler(args.source).await?;
        Ok(match compiler.compile().await? {
            Some(mut runner) => {
                let judger = runner.run((args.mem, args.cpu), args.input).await?;

                todo!("stream output");

                let stat = judger.stat();

                Some(todo!())
            }
            None => None,
        })
    }
    pub fn get_memory_reserved(&self, mem: u64) -> u64 {
        self.spec.get_memory_reserved_size(mem)
    }
}
