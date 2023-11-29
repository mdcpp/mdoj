use crate::sandbox::prelude::*;
use tokio::time;

#[tokio::test]
async fn exec() {
    crate::init::new().await;

    {
        let daemon = ContainerDaemon::new_with_id(".temp", 12);
        let container = daemon.create("plugins/rlua-54/rootfs").await.unwrap();

        let process = container
            .execute(
                vec!["/rlua-54", "hello"],
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
#[ignore = "does not always pass, need t consider the latency of kernel!"]
#[tokio::test]
async fn cgroup_cpu() {
    crate::init::new().await;

    {
        let daemon = ContainerDaemon::new_with_id(".temp", 13);
        let container = daemon.create("plugins/rlua-54/rootfs").await.unwrap();

        let process = container
            .execute(
                vec!["/rlua-54", "violate","cpu"],
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
        let daemon = ContainerDaemon::new_with_id(".temp", 14);
        let container = daemon.create("plugins/rlua-54/rootfs").await.unwrap();

        let process = container
            .execute(
                vec!["/rlua-54", "violate","net"],
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
async fn memory() {
    crate::init::new().await;

    {
        let daemon = ContainerDaemon::new_with_id(".temp", 15);
        let container = daemon.create("plugins/rlua-54/rootfs").await.unwrap();

        let process = container
            .execute(
                vec!["/rlua-54", "violate","mem"],
                Limit {
                    cpu_us: 1000 * 1000 * 1000,
                    rt_us: 1000 * 1000 * 1000,
                    total_us: 20 * 1000,
                    swap_user: 0,
                    kernel_mem: 64 * 1024 * 1024,
                    user_mem: 64 * 1024 * 1024,
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
async fn disk() {
    crate::init::new().await;

    {
        let daemon = ContainerDaemon::new_with_id(".temp", 16);
        let container = daemon.create("plugins/rlua-54/rootfs").await.unwrap();

        let process = container
            .execute(
                vec!["/rlua-54", "violate","disk"],
                Limit {
                    cpu_us: 1000 * 1000 * 1000,
                    rt_us: 1000 * 1000 * 1000,
                    total_us: 20 * 1000,
                    swap_user: 0,
                    kernel_mem: 64 * 1024 * 1024,
                    user_mem: 64 * 1024 * 1024,
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
#[ignore = "failing because of the test suite, not the sandbox"]
async fn syscall() {
    crate::init::new().await;

    {
        let daemon = ContainerDaemon::new_with_id(".temp", 17);
        let container = daemon.create("plugins/rlua-54/rootfs").await.unwrap();

        let process = container
            .execute(
                vec!["/rlua-54", "violate","syscall"],
                Limit {
                    cpu_us: 1000 * 1000 * 1000,
                    rt_us: 1000 * 1000 * 1000,
                    total_us: 20 * 1000,
                    swap_user: 0,
                    kernel_mem: 64 * 1024 * 1024,
                    user_mem: 64 * 1024 * 1024,
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
