pub mod prison;
pub mod proc;
pub mod unit;
pub mod utils;

use thiserror::Error;



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

#[derive(Debug)]
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
    use crate::limit::prison::Prison;
    use tokio::time;

    use super::*;

    #[tokio::test]
    async fn exec() {
        crate::init::new().await;

        {
            let prison = Prison::new(".temp");
            let cell = prison.create("plugins/lua-5.2/rootfs").await.unwrap();

            let process = cell
                .execute(
                    &vec!["/usr/local/bin/lua", "/test.lua"],
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

            println!("{}",process.status);

            assert!(process.succeed());

            let out=process.stdout;
            assert_eq!(out, b"hello world\n");
        }

        // unlike async-std, tokio won't wait for all background task to finish before exit
        time::sleep(time::Duration::from_millis(12)).await;
    }
}
