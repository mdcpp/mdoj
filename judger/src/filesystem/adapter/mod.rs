mod error;
mod fuse;
mod handle;
mod reply;
mod template;

pub use fuse::Filesystem;
pub use template::Template;

#[cfg(test)]
mod test {
    use super::*;
    use crate::semaphore::Semaphore;
    use env_logger::*;

    #[tokio::test]
    #[ignore = "not meant to be tested"]
    async fn test_mount() {
        Builder::from_default_env()
            .filter_level(log::LevelFilter::Trace)
            .try_init()
            .ok();

        log::info!("mounting test tarball in .temp/1 ...");
        let global_resource = Semaphore::new(4096 * 1024 * 1024, 1);
        let template = Template::new("test/nested.tar").await.unwrap();
        let filesystem = template
            .as_filesystem(
                global_resource
                    .get_permit(1024 * 1024 * 1024)
                    .await
                    .unwrap(),
            )
            .await;
        let mut mount_handle = filesystem.mount("./.temp/1").await.unwrap();
        let handle = &mut mount_handle;

        tokio::select! {
            res = handle => res.unwrap(),
            _ = tokio::signal::ctrl_c() => {
                mount_handle.unmount().await.unwrap()
            }
        }
    }
}
