use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering}, fs::Permissions, os::unix::fs::PermissionsExt,
};

use tokio::fs;

use crate::init::config::CONFIG;

use super::{container::Container, utils::semaphore::MemorySemaphore, Error};

// Container daemon, manage container creation and deletion
// setup and clean tmp files, reverse memory through semaphore
pub struct ContainerDaemon {
    id_counter: AtomicU64,
    pub(super) memory_counter: MemorySemaphore,
    pub(super) tmp: PathBuf,
}

impl ContainerDaemon {
    pub fn new(tmp: impl AsRef<Path>) -> Self {
        let config = CONFIG.get().unwrap();
        Self {
            id_counter: Default::default(),
            memory_counter: MemorySemaphore::new(config.platform.available_memory),
            tmp: tmp.as_ref().to_path_buf(),
        }
    }
    #[cfg(test)]
    pub fn new_with_id(tmp: impl AsRef<Path>, id: u64) -> Self {
        let config = CONFIG.get().unwrap();
        Self {
            id_counter: AtomicU64::new(id),
            memory_counter: MemorySemaphore::new(config.platform.available_memory),
            tmp: tmp.as_ref().to_path_buf(),
        }
    }
    pub async fn create(&self, root: impl AsRef<Path>) -> Result<Container<'_>, Error> {
        let id = self.id_counter.fetch_add(1, Ordering::Acquire).to_string();
        log::trace!("Creating new container: {}", id);
        let container_root = self.tmp.join(id.clone());

        fs::create_dir(container_root.clone()).await?;
        fs::create_dir(container_root.clone().join("src")).await?;
        // fs::set_permissions(container_root.clone().join("src"), Permissions::from_mode(0o777)).await?;

        Ok(Container {
            id,
            daemon: self,
            root: root.as_ref().to_path_buf(),
        })
    }
}
