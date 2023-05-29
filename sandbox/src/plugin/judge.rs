use std::path::Path;

use crate::jail::{
    jail::{Cell, Prison},
    limit::CpuLimit,
};

use super::spec::PluginSpec;

macro_rules! report {
    ($e:expr) => {
        match $e {
            Err(e) => {
                log::error!("{}", e);
                return None;
            }
            Ok(x) => x,
        }
    };
}

pub struct Judger {
    prison: Prison,
}

impl Judger {
    pub fn new(tmp: impl AsRef<Path>) -> Self {
        Self {
            prison: Prison::new(tmp),
        }
    }
    pub async fn create<'a>(&'a self, spec: &'a PluginSpec, limit: Limit) -> Option<Judge<'a>> {
        let cell = report!(self.prison.create(spec.root()).await);
        Some(Judge { limit, spec, cell })
    }
}

pub struct Limit {
    pub cpu_us: u64,
    pub mem: i64,
}

pub struct Judge<'a> {
    limit: Limit,
    spec: &'a PluginSpec,
    cell: Cell<'a>,
}

impl<'a> Judge<'a> {
    pub async fn compile(&self, source_code: Vec<u8>) -> Option<()> {
        let mut proc = report!(self
            .cell
            .execute(&self.spec.compile.args(), self.spec.compile.limit()));

        report!(proc.write_all(source_code).await);

        let status = proc.wait().await.succeed();

        let stdout = report!(proc.read_all().await);

        let stdout = String::from_utf8_lossy(&stdout);

        stdout.split("\n").for_each(|message| {
            let content = &message[1..message.len()];
            if let Some(level) = message.chars().next() {
                match level {
                    '5' => log::error!("{}", content),
                    '4' => log::warn!("{}", content),
                    '3' => log::info!("{}", content),
                    '2' => log::debug!("{}", content),
                    _ => log::trace!("{}", content),
                }
            }
        });

        match status {
            true => Some(()),
            false => None,
        }
    }
    pub async fn execute_task(&self, input: Vec<u8>) -> Option<TaskChecker> {
        let mut proc = report!(self.cell.execute(
            &self.spec.compile.args(),
            self.spec.execute.limit(self.limit.cpu_us, self.limit.mem)
        ));

        report!(proc.write_all(input).await);

        let status = proc.wait().await;

        let cpu_usage = proc.cpu_usage();

        match status.succeed() {
            true => Some(TaskChecker {
                cpu_usage,
                output: report!(proc.read_all().await),
            }),
            false => None,
        }
    }
}

pub struct TaskChecker {
    pub cpu_usage: CpuLimit,
    output: Vec<u8>,
}

pub enum CheckMethod {
    ExactSame = 0,      // exactly same
    ExactNewline = 1,   // filter out empty line, but each line should be exactly same
    SpaceOrNewline = 2, // treat one blank, newline as one space, match word even it's zero-sized
    SpaceNewline = 3, // treat one blank, double blank, newline (or a mixture of them) as one space, match non-blank word
}

impl TaskChecker {
    pub fn check(&self, out: Vec<u8>, method: CheckMethod) -> bool {
        match method {
            CheckMethod::ExactSame => out.iter().eq(self.output.iter()),
            CheckMethod::ExactNewline => todo!(),
            CheckMethod::SpaceOrNewline => todo!(),
            CheckMethod::SpaceNewline => todo!(),
        }
    }
}
