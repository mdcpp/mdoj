use std::fmt::Display;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    select, time,
};

use crate::{init::config::CONFIG, sandbox::utils::limiter::LimitReason};

use super::{
    utils::{
        limiter::{cpu::CpuStatistics, mem::MemStatistics, Limiter},
        nsjail::NsJail,
        preserve::MemoryPermit,
    },
    Error,
};

// const BUFFER_LIMIT: usize = 32 * 1024 * 1024 - 1;

pub struct RunningProc {
    pub(super) limiter: Limiter,
    pub(super) nsjail: NsJail,
    pub(super) _memory_holder: MemoryPermit,
}

impl RunningProc {
    pub async fn write_all(&mut self, buf: &Vec<u8>) -> Result<(), Error> {
        let mut child = self.nsjail.process.as_ref().unwrap().lock().await;
        let stdin = child.stdin.as_mut().ok_or(Error::CapturedPipe)?;

        stdin.write_all(&buf).await?;

        Ok(())
    }
    pub async fn wait(mut self) -> Result<ExitProc, Error> {
        let config = CONFIG.get().unwrap();

        let status = select! {
            reason = self.limiter.wait_exhausted()=>{
                match reason.unwrap(){
                    LimitReason::Cpu=>ExitStatus::CpuExhausted,
                    LimitReason::Mem=>ExitStatus::MemExhausted
                }
            }
            code = self.nsjail.wait()=>{
                match code{
                    Some(x)=>ExitStatus::Code(x),
                    None=>ExitStatus::SigExit
                }
            }
            _ = time::sleep(time::Duration::from_secs(3600))=>{
                return Err(Error::Stall);
            }
        };

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

        Ok(ExitProc {
            status,
            stdout: buf.to_vec(),
            cpu,
            mem,
        })
    }
}

pub struct ExitProc {
    pub status: ExitStatus,
    pub stdout: Vec<u8>,
    pub cpu: CpuStatistics,
    pub mem: MemStatistics,
}

impl ExitProc {
    pub fn succeed(&self) -> bool {
        match self.status {
            ExitStatus::Code(x) => x == 0,
            _ => false,
        }
    }
}

pub enum ExitStatus {
    SigExit, // RuntimeError
    Code(i32),
    MemExhausted,
    CpuExhausted,
}

impl Display for ExitStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExitStatus::SigExit => write!(f, "Killed by signal"),
            ExitStatus::Code(x) => write!(f, "Exit with code {}", x),
            ExitStatus::MemExhausted => write!(f, "Reach memory limit"),
            ExitStatus::CpuExhausted => write!(f, "Reach cpu quota"),
        }
    }
}
