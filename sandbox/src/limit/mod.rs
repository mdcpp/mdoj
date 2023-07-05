pub mod prison;
pub mod proc;
pub mod unit;
pub mod utils;

use thiserror::Error;

use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicI64, AtomicPtr, Ordering},
        Arc,
    },
};

use tokio::fs;

use crate::{init::config::CONFIG, jail::resource::ResourceCounter};

// use self::{nsjail::NsJail, preserve::MemoryHolder};

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
    #[error("Impossible to run the task given the provided resource preservation policy")]
    InsufficientResource,
}

// const NICE: i32 = 18;

pub struct Limit {
    pub lockdown: bool,
    pub cpu_us: u64,
    pub rt_us: i64,
    pub total_us: u64,
    pub user_mem: i64,
    pub kernel_mem: i64,
    pub swap_user: i64,
}

// impl Limit {
//     pub fn max() -> Self {
//         Self {
//             lockdown: true,
//             cpu_us: u64::MAX / 2 - 1,
//             rt_us: i64::MAX / 2 - 1,
//             total_us: u64::MAX / 2 - 1,
//             user_mem: i64::MAX / 2 - 1,
//             kernel_mem: i64::MAX / 2 - 1,
//             swap_user: 0,
//         }
//     }
// }

// // pub struct Process<'a> {
// //     process: Option<Child>,
// //     // limiter: Limiter,
// //     resource_guard: MemoryHolder<'a>,
// // }

// // #[derive(PartialEq, Eq, Debug)]
// // pub enum ProcessStatus {
// //     Exit(i32),
// //     Exhaust(LimitReason),
// //     SigExit,
// //     Stall,
// // }

// // impl ProcessStatus {
// //     pub fn succeed(&self) -> bool {
// //         match self {
// //             ProcessStatus::Exit(x) => *x == 0,
// //             _ => false,
// //         }
// //     }
// // }

// // impl<'a> Drop for Process<'a> {
// //     fn drop(&mut self) {
// //         let mut process = self.process.take().unwrap();
// //         tokio::spawn(async move {
// //             process.kill().await.unwrap();
// //             process.wait().await.unwrap();
// //         });
// //     }
// // }

// // impl<'a> Process<'a> {
// //     pub async fn kill(&mut self) {
// //         self.process.take().unwrap().kill().await.ok();
// //     }
// //     pub fn stdin(&mut self) -> Option<ChildStdin> {
// //         self.process.as_mut().unwrap().stdin.take()
// //     }
// //     pub fn stdout(&mut self) -> Option<ChildStdout> {
// //         self.process.as_mut().unwrap().stdout.take()
// //     }
// //     pub fn stderr(&mut self) -> Option<ChildStderr> {
// //         self.process.as_mut().unwrap().stderr.take()
// //     }
// //     pub async fn wait(&mut self) -> ProcessStatus {
// //         select! {
// //             x = self.process.as_mut().unwrap().wait() => {
// //                 match x.unwrap().code() {
// //                     Some(x) => ProcessStatus::Exit(x),
// //                     None => ProcessStatus::SigExit
// //                 }
// //             }
// //             _ = self.limiter.wait() => {
// //                 ProcessStatus::Exhaust(self.limiter.status().unwrap())
// //             }
// //             _ = time::sleep(time::Duration::from_secs(3600)) => {
// //                 ProcessStatus::Stall
// //             }
// //         }
// //     }
// //     pub async fn write_all(&mut self, buf: Vec<u8>) -> Result<(), Error> {
// //         self.process
// //             .as_mut()
// //             .unwrap()
// //             .stdin
// //             .as_mut()
// //             .ok_or(Error::CapturedPiped)?
// //             .write_all(&buf)
// //             .await?;
// //         Ok(())
// //     }
// //     pub async fn read_all(&mut self) -> Result<Vec<u8>, Error> {
// //         let mut buf = Vec::new();
// //         self.process
// //             .as_mut()
// //             .unwrap()
// //             .stdout
// //             .as_mut()
// //             .ok_or(Error::CapturedPiped)?
// //             .read_to_end(&mut buf)
// //             .await?;
// //         Ok(buf)
// //     }
// //     pub fn cpu_usage(&mut self) -> CpuLimit {
// //         self.limiter.cpu_usage()
// //     }
// // }
