use thiserror::Error;

pub mod cpuacct;
pub mod jail;
pub mod limit;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: `{0}`")]
    IO(#[from] std::io::Error),
    #[error("`{0}`")]
    ControlGroup(#[from] cgroups_rs::error::Error),
    #[error("The pipe has been capture")]
    CapturedPiped,
    #[error("Error from system call `{0}`")]
    Libc(u32),
    #[error("Resource provided, but the process refused to continue")]
    Stall,
    #[error("Fail calling cgroup, check subsystem and hier support")]
    CGroup,
}
