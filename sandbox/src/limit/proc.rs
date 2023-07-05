use std::sync::{atomic::AtomicPtr, Arc};

use super::utils::{
    limiter::{cpu::CpuStatistics, mem::MemStatistics, Limiter},
    nsjail::NsJail,
    preserve::MemoryHolder,
};

pub struct RunningProc {
    pub(super) limiter: Limiter,
    pub(super) state: Arc<ProcState>,
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
    ExitSig,
    Code(i16),
    MemExhausted,
    CpuExhausted,
}
