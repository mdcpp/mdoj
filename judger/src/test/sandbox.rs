#[cfg(test)]
mod test {
    use crate::sandbox::prelude::*;
    use tokio::time;

    #[tokio::test]
    async fn exec() {
        crate::init::new().await;

        {
            let daemon = ContainerDaemon::new_with_id(".temp", 2);
            let container = daemon.create("plugins/lua-5.2/rootfs").await.unwrap();

            let process = container
                .execute(
                    vec!["/usr/local/bin/lua", "/test/test1.lua"],
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
            let daemon = ContainerDaemon::new_with_id(".temp", 3);
            let container = daemon.create("plugins/lua-5.2/rootfs").await.unwrap();

            let process = container
                .execute(
                    vec!["/usr/local/bin/lua", "/test/test2.lua"],
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

            assert_eq!(process.status, ExitStatus::CpuExhausted);

            assert!(!process.succeed());
        }

        // unlike async-std, tokio won't wait for all background task to finish before exit
        time::sleep(time::Duration::from_millis(12)).await;
    }
    #[tokio::test]
    async fn network() {
        crate::init::new().await;

        {
            let daemon = ContainerDaemon::new_with_id(".temp", 4);
            let container = daemon.create("plugins/lua-5.2/rootfs").await.unwrap();

            let process = container
                .execute(
                    vec!["/usr/local/bin/lua", "/test/test3.lua"],
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
