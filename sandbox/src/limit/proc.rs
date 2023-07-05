use std::sync::{atomic::AtomicPtr, Arc};

use tokio::select;

use crate::limit::utils::limiter::LimitReason;

use super::utils::{
    limiter::{cpu::CpuStatistics, mem::MemStatistics, Limiter},
    nsjail::NsJail,
    preserve::MemoryHolder,
};

pub struct RunningProc {
    pub(super) limiter: Limiter,
    pub(super) nsjail: NsJail,
    pub(super) memory_holder: MemoryHolder,
}

impl RunningProc {
    pub async fn wait(mut self) -> ExitProc {
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
        };
        todo!()
    }
}

impl RunningProc {}

pub struct ExitProc {
    status: ExitStatus,
    stdout: Vec<u8>,
}

pub struct ProcState {
    pub(super) nsjail: NsJail,
    pub(super) memory_holder: MemoryHolder,
}

pub enum ExitStatus {
    SigExit,
    Code(i32),
    MemExhausted,
    CpuExhausted,
}
