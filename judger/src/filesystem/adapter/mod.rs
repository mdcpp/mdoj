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
    // use crate::semaphore::Semaphore;
    use env_logger::*;

    #[tokio::test]
    #[ignore = "not meant to be tested"]
    async fn test_mount() {
        Builder::from_default_env()
            .filter_level(log::LevelFilter::Trace)
            .try_init()
            .ok();

        log::info!("mounting test tarball in .temp ...");
        let template = Template::new("plugins/rlua-54.lang").await.unwrap();
        let filesystem = template.as_filesystem(1024 * 1024 * 1024);
        let mut mount_handle = filesystem.raw_mount_with_path("./.temp/5").await.unwrap();
        let handle = &mut mount_handle;

        tokio::select! {
            res = handle => res.unwrap(),
            _ = tokio::signal::ctrl_c() => {
                mount_handle.unmount().await.unwrap()
            }
        }
    }
}
