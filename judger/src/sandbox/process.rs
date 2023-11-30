use std::fmt::Display;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    select, time,
};

use crate::{init::config::CONFIG, sandbox::utils::limiter::LimitReason};

use super::{
    utils::{
        limiter::{cpu::CpuStatistics, mem::MemStatistics, Limiter},
        nsjail::{NsJail, TermStatus},
        semaphore::MemoryPermit,
    },
    Error,
};

impl From<LimitReason> for ExitStatus {
    fn from(value: LimitReason) -> Self {
        match value {
            LimitReason::Cpu => ExitStatus::CpuExhausted,
            LimitReason::Mem => ExitStatus::MemExhausted,
            LimitReason::SysMem => ExitStatus::SysError,
        }
    }
}

impl From<TermStatus> for ExitStatus {
    fn from(value: TermStatus) -> Self {
        match value {
            TermStatus::SigExit(x) => ExitStatus::SigExit(x),
            TermStatus::Code(x) => ExitStatus::Code(x),
        }
    }
}

// an abstraction of running process, no meaningful logic implemented
pub struct RunningProc {
    pub(super) limiter: Limiter,
    pub(super) nsjail: NsJail,
    pub(super) _memory_holder: MemoryPermit,
}

impl RunningProc {
    pub async fn write_all(&mut self, buf: &[u8]) -> Result<(), Error> {
        let mut child = self.nsjail.process.as_ref().unwrap().lock().await;
        let stdin = child.stdin.as_mut().ok_or(Error::CapturedPipe)?;

        stdin.write_all(buf).await?;

        stdin.shutdown().await?;

        Ok(())
    }
    pub async fn wait(mut self) -> Result<ExitProc, Error> {
        let config = CONFIG.get().unwrap();

        let mut status: ExitStatus = select! {
            reason = self.limiter.wait_exhausted()=>reason.unwrap().into(),
            code = self.nsjail.wait()=> code.into(),
            _ = time::sleep(time::Duration::from_secs(3600))=>{
                return Err(Error::Stall);
            }
        };
        // because in the senario of out of memory, process will be either exit with code
        // 11(unable to allocate memory) or kill by signal(if oom killer is enable)
        // , so we need to check if it is oom
        if self.limiter.check_oom() {
            status = ExitStatus::MemExhausted;
        }

        let mut child = self.nsjail.process.as_ref().unwrap().lock().await;
        let mut stdout = child
            .stdout
            .as_mut()
            .ok_or(Error::CapturedPipe)?
            .take((config.platform.output_limit) as u64);

        let mut buf = Vec::with_capacity(256);

        stdout.read_to_end(&mut buf).await?;

        if stdout.into_inner().read_u8().await.is_ok() {
            return Err(Error::BufferFull);
        }

        let (cpu, mem) = self.limiter.status().await;
        let output_limit = config.platform.output_limit as u64;

        let _memory_holder = self._memory_holder.downgrade(output_limit);
        Ok(ExitProc {
            status,
            stdout: buf.to_vec(),
            cpu,
            mem,
            _memory_holder,
        })
    }
}

pub struct ExitProc {
    pub status: ExitStatus,
    pub stdout: Vec<u8>,
    pub cpu: CpuStatistics,
    pub mem: MemStatistics,
    _memory_holder: MemoryPermit,
}

impl ExitProc {
    pub fn succeed(&self) -> bool {
        match self.status {
            ExitStatus::Code(x) => x == 0 || x == 255,
            _ => false,
        }
    }
}

#[derive(Debug, PartialEq, PartialOrd, Ord, Eq)]
pub enum ExitStatus {
    SigExit(i32), // RuntimeError
    Code(i32),
    MemExhausted,
    CpuExhausted,
    SysError,
}

impl Display for ExitStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExitStatus::SigExit(x) => write!(f, "Killed by signal {}", x),
            ExitStatus::Code(x) => write!(f, "Exit with code {}", x),
            ExitStatus::MemExhausted => write!(f, "Reach memory limit"),
            ExitStatus::CpuExhausted => write!(f, "Reach cpu quota"),
            ExitStatus::SysError => write!(f, "Unknown system error"),
        }
    }
}
