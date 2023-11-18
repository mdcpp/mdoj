pub(super) mod container;
pub(super) mod daemon;
pub(super) mod process;
pub(super) mod utils;

use thiserror::Error;

pub mod prelude {
    pub use super::container::Container;
    pub use super::daemon::ContainerDaemon;
    pub use super::process::{ExitProc, ExitStatus, RunningProc};
    pub use super::utils::limiter::cpu::CpuStatistics;
    pub use super::utils::limiter::mem::MemStatistics;
    pub use super::utils::semaphore::{MemoryPermit, MemorySemaphore, MemoryStatistic};
    pub use super::Error;
    pub use super::Limit;
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("Impossible to run the task given the provided resource preservation policy")]
    ImpossibleResource,
    #[error("Resource provided, but the process refused to continue")]
    Stall,
    #[error("The pipe has been capture")]
    CapturedPipe,
    #[error("IO error: `{0}`")]
    IO(#[from] std::io::Error),
    #[error("`{0}`")]
    ControlGroup(#[from] cgroups_rs::error::Error),
    #[error("Error from system call `{0}`")]
    Libc(u32),
    #[error("Fail calling cgroup, check subsystem and hier support")]
    CGroup,
    #[error("Read buffer is full before meeting EOF")]
    BufferFull,
}

// const NICE: i32 = 18;

#[derive(Debug, Clone)]
pub struct Limit {
    pub lockdown: bool,
    pub cpu_us: u64,
    pub rt_us: u64,
    pub total_us: u64,
    pub user_mem: u64,
    pub kernel_mem: u64,
    pub swap_user: u64,
}
