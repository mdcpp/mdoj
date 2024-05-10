mod error;
mod monitor;
mod process;

use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    time::Duration,
};

pub use self::monitor::{Cpu, Memory};
pub use error::Error;
/// Context of the sandbox
///
/// define resource limit and filesystem is out of the scope of `filesystem`
pub trait Context: Limit {
    type FS: Filesystem;
    fn create_fs(&mut self) -> Self::FS;
    fn get_args(&mut self) -> impl Iterator<Item = &OsStr>;
}

pub trait Limit {
    fn get_cpu(&mut self) -> Cpu;
    fn get_memory(&mut self) -> Memory;
    fn get_output(&mut self) -> u64;
    fn get_walltime(&mut self) -> Duration {
        Duration::from_secs(60 * 30)
    }
}

pub trait Filesystem {
    fn mount(&mut self) -> impl AsRef<Path> + Send;
}

impl Filesystem for PathBuf {
    fn mount(&mut self) -> impl AsRef<Path> + Send {
        self.as_path().iter()
    }
}

impl Limit for (Cpu, Memory, u64, Duration) {
    fn get_cpu(&mut self) -> Cpu {
        self.0.clone()
    }
    fn get_memory(&mut self) -> Memory {
        self.1.clone()
    }
    fn get_output(&mut self) -> u64 {
        self.2
    }
    fn get_walltime(&mut self) -> Duration {
        self.3
    }
}
