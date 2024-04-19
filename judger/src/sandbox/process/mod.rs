pub(super) mod nsjail;
use std::{
    ffi::{OsStr, OsString},
    path::PathBuf,
    str::FromStr,
};

use crate::error::Error;
use tokio::process::*;

use super::{daemon::*, *};

lazy_static::lazy_static! {
    pub static ref NSJAIL_PATH: PathBuf =PathBuf::from("./nsjail-3.1");
    pub static ref NSJAIL_ARGS: Vec<OsString> = vec![
        OsString::from("--disable_clone_newuser"),
        OsString::from("--disable_clone_newuser"),
        OsString::from("--disable_clone_newcgroup"),
        OsString::from("--env"),
        OsString::from("PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin")
    ];
    // FIXME: respect config
}

pub trait Context {
    fn get_cpu(&self) -> Cpu;
    fn get_memory(&self) -> Memory;
    fn create_fs(&self) -> impl AsRef<std::path::Path> + Send;
    fn destroy_fs(&self, fs: impl AsRef<std::path::Path> + Send);
    fn get_args(&self) -> &[OsString];
}

impl Daemon {
    pub async fn spawn<C: Context>(&self, context: C) -> Result<Process<C>, Error> {
        let memory = context.get_memory().get_reserved();

        let handle = self.spawn_handle(memory).await?;

        Ok(Process { context, handle })
    }
}

/// A inactive process with resource allocated
pub struct Process<C: Context> {
    context: C,
    handle: handle::Handle,
}

impl<C: Context> Process<C> {
    pub async fn wait(&mut self) {
        let mem = self.context.get_memory();
        let cpu = self.context.get_cpu();
        let rootfs = self.context.create_fs();
        let limiter = limiter::Limiter::new_mount(self.handle.get_cg_name(), cpu, mem).unwrap();

        let mut args = todo!();

        // let process=Command::new(NSJAIL_PATH.as_path())
        //     .args(&args)
        //     .spawn()
        //     .unwrap();
    }
}

// impl<C: Context> Drop for Process<C> {
//     fn drop(&mut self) {
//         self.context.destroy_fs(self.rootfs.as_path());
//     }
// }
