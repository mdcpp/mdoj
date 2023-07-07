use std::path::Path;

use crate::{
    init::config::CONFIG,
    limit::{prison::Prison, unit::Unit, utils::limiter::cpu::CpuStatistics},
};

use super::{spec::LangSpec, JudgeStatus};

macro_rules! report {
    ($e:expr,$s:ident) => {
        match $e {
            Err(e) => {
                log::error!("{}", e);
                return Err(JudgeStatus::$s);
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
    pub async fn create<'a>(
        &'a self,
        spec: &'a LangSpec,
        limit: Limit,
    ) -> Result<Task<'a>, JudgeStatus> {
        let cell = report!(self.prison.create(spec.root()).await, Unrecoverable);
        Ok(Task { limit, spec, cell })
    }
    // pub fn usage(&self) -> ResourceUsage {
    //     self.prison.usage()
    // }
}

pub struct Limit {
    pub cpu_us: u64,
    pub mem: i64,
}

pub struct Task<'a> {
    limit: Limit,
    spec: &'a LangSpec,
    cell: Unit<'a>,
}

impl<'a> Task<'a> {
    pub async fn compile(&self, source_code: Vec<u8>) -> Result<(), JudgeStatus> {
        let mut proc = report!(
            self.cell
                .execute(&self.spec.compile.args(), self.spec.compile.limit())
                .await,
            CompileError
        );

        report!(proc.write_all(source_code).await, CompileError);

        let proc = report!(proc.wait().await, CompileError);

        let succeed = proc.succeed();

        let stdout = proc.stdout;

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

        Err(match succeed {
            true => JudgeStatus::CompileError,
            false => JudgeStatus::Compiling,
        })
    }
    pub async fn execute_task(&self, input: Vec<u8>) -> Result<TaskResult, JudgeStatus> {
        let config = CONFIG.get().unwrap();
        let mut proc = report!(
            self.cell
                .execute(
                    &self.spec.compile.args(),
                    self.spec.execute.limit(self.limit.cpu_us, self.limit.mem)
                )
                .await,
            RuntimeError
        );

        report!(proc.write_all(input).await, RuntimeError);

        let proc = report!(proc.wait().await, RuntimeError);

        let succeed = proc.succeed();

        let cpu_usage = proc.cpu;

        match succeed {
            true => Ok(TaskResult {
                cpu_usage,
                output: proc.stdout,
            }),
            false => Err(JudgeStatus::WrongAnswer),
        }
    }
}

pub struct TaskResult {
    pub cpu_usage: CpuStatistics,
    output: Vec<u8>,
}

pub enum CheckMethod {
    ExactSame = 0,      // exactly same
    ExactNewline = 1,   // filter out empty line, but each line should be exactly same
    SpaceOrNewline = 2, // treat one blank, newline as one space, match word even it's zero-sized
    SpaceNewline = 3, // treat one blank, double blank, newline (or a mixture of them) as one space, match non-blank word
}

impl TaskResult {
    pub fn check(&self, out: Vec<u8>, method: CheckMethod) -> bool {
        match method {
            CheckMethod::ExactSame => out.iter().eq(self.output.iter()),
            CheckMethod::ExactNewline => todo!(),
            CheckMethod::SpaceOrNewline => todo!(),
            CheckMethod::SpaceNewline => todo!(),
        }
    }
}
