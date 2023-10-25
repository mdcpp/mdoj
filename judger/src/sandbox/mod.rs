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
    pub use super::utils::preserve::{MemoryCounter, MemoryHolder, MemoryStatistic};
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
    pub rt_us: i64,
    pub total_us: u64,
    pub user_mem: i64,
    pub kernel_mem: i64,
    pub swap_user: i64,
}

#[cfg(test)]
mod test {
    use crate::sandbox::daemon::ContainerDaemon;
    use tokio::time;

    use super::*;

    #[tokio::test]
    async fn exec() {
        crate::init::new().await;

        {
            let daemon = ContainerDaemon::new(".temp");
            let container = daemon.create("plugins/lua-5.2/rootfs").await.unwrap();

            let process = container
                .execute(
                    vec!["/usr/local/bin/lua", "/test.lua"],
                    Limit {
                        cpu_us: 1000 * 1000 * 1000,
                        rt_us: 1000 * 1000 * 1000,
                        total_us: 20 * 1000,
                        swap_user: 0,
                        kernel_mem: 128 * 1024 * 1024,
                        user_mem: 512 * 1024 * 1024,
                        lockdown: false,
                    },
                )
                .await
                .unwrap();

            let process = process.wait().await.unwrap();

            assert!(process.succeed());

            let out = process.stdout;
            assert_eq!(out, b"hello world\n");
        }

        // unlike async-std, tokio won't wait for all background task to finish before exit
        time::sleep(time::Duration::from_millis(12)).await;
    }
    #[tokio::test]
    async fn cgroup_cpu() {
        crate::init::new().await;

        {
            let daemon = ContainerDaemon::new(".temp");
            let container = daemon.create("plugins/lua-5.2/rootfs").await.unwrap();

            let process = container
                .execute(
                    vec![
                        "/usr/local/bin/lua",
                        "/test/test2.lua",
                    ],
                    Limit {
                        cpu_us: 1000 * 1000 * 1000,
                        rt_us: 1000 * 1000 * 1000,
                        total_us: 20 * 1000,
                        swap_user: 0,
                        kernel_mem: 128 * 1024 * 1024,
                        user_mem: 512 * 1024 * 1024,
                        lockdown: false,
                    },
                )
                .await
                .unwrap();

            let process = process.wait().await.unwrap();

            assert!(!process.succeed());
        }

        // unlike async-std, tokio won't wait for all background task to finish before exit
        time::sleep(time::Duration::from_millis(12)).await;
    }
    #[tokio::test]
    async fn network() {
        crate::init::new().await;

        {
            let daemon = ContainerDaemon::new(".temp");
            let container = daemon.create("plugins/lua-5.2/rootfs").await.unwrap();

            let process = container
                .execute(
                    vec![
                        "/usr/local/bin/lua",
                        "/test/test3.lua",
                    ],
                    Limit {
                        cpu_us: 1000 * 1000 * 1000,
                        rt_us: 1000 * 1000 * 1000,
                        total_us: 20 * 1000,
                        swap_user: 0,
                        kernel_mem: 128 * 1024 * 1024,
                        user_mem: 512 * 1024 * 1024,
                        lockdown: false,
                    },
                )
                .await
                .unwrap();

            let process = process.wait().await.unwrap();

            assert!(!process.succeed());
        }

        // unlike async-std, tokio won't wait for all background task to finish before exit
        time::sleep(time::Duration::from_millis(12)).await;
    }
}
