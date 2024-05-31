use super::adapter::Filesystem;

use tokio::io::{AsyncRead, AsyncSeek};

use super::mkdtemp::MkdTemp;

pub struct MountHandle(Option<fuse3::raw::MountHandle>, Option<MkdTemp>);

impl MountHandle {
    pub fn get_path(&self) -> &std::path::Path {
        self.1.as_ref().unwrap().get_path()
    }
}

impl Drop for MountHandle {
    fn drop(&mut self) {
        let handle = self.0.take().unwrap();
        let mountpoint = self.1.take().unwrap();
        tokio::spawn(async move {
            #[cfg(debug_assertions)]
            {
                log::warn!("debug mode: wait for 120s before drop mountpoint");
                tokio::time::sleep(tokio::time::Duration::from_secs(120)).await;
            }
            handle.unmount().await.unwrap();
            drop(mountpoint);
        });
    }
}

impl<F> Filesystem<F>
where
    F: AsyncRead + AsyncSeek + Unpin + Send + Sync + 'static,
{
    pub async fn mount(self) -> std::io::Result<MountHandle> {
        let mountpoint = MkdTemp::new();
        let handle = self.raw_mount_with_path(mountpoint.get_path()).await?;
        Ok(MountHandle(Some(handle), Some(mountpoint)))
    }
}
