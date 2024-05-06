// mod limiter;
mod monitor;
mod process;

use std::{ffi::OsStr, path::Path, time::Duration};

pub use self::monitor::{Cpu, Memory};

pub trait Context: Limit {
    type FS: Filesystem;
    fn create_fs(&mut self) -> Self::FS;
}

pub trait Limit {
    fn get_cpu(&mut self) -> Cpu;
    fn get_memory(&mut self) -> Memory;
    fn get_args(&mut self) -> impl Iterator<Item = &OsStr>;
    fn get_output_limit(&mut self) -> u64;
    fn get_walltime(&mut self) -> Duration {
        Duration::from_secs(60 * 30)
    }
}

pub trait Filesystem {
    fn mount(&mut self) -> impl AsRef<Path> + Send;
    fn get_size(&mut self) -> u64;
}
