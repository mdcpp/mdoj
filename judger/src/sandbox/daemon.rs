use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicI64, Ordering},
};

use tokio::fs;

use crate::init::config::CONFIG;

use super::{container::Container, utils::preserve::MemoryCounter, Error};

pub struct ContainerDaemon {
    id_counter: AtomicI64,
    pub(super) memory_counter: MemoryCounter,
    pub(super) tmp: PathBuf,
}

impl ContainerDaemon {
    pub fn new(tmp: impl AsRef<Path>) -> Self {
        let config = CONFIG.get().unwrap();
        Self {
            id_counter: Default::default(),
            memory_counter: MemoryCounter::new(config.platform.available_memory),
            tmp: tmp.as_ref().to_path_buf(),
        }
    }
    #[cfg(test)]
    pub fn new_with_id(tmp: impl AsRef<Path>,id:i64) -> Self {
        let config = CONFIG.get().unwrap();
        Self {
            id_counter: AtomicI64::new(id),
            memory_counter: MemoryCounter::new(config.platform.available_memory),
            tmp: tmp.as_ref().to_path_buf(),
        }
    }
    pub async fn create<'a>(&'a self, root: impl AsRef<Path>) -> Result<Container<'a>, Error> {
        log::trace!("Creating new container daemon");
        let id = self.id_counter.fetch_add(1, Ordering::Release).to_string();
        let container_root = self.tmp.join(id.clone());

        fs::create_dir(container_root.clone()).await?;
        fs::create_dir(container_root.clone().join("src")).await?;

        Ok(Container {
            id,
            daemon: self,
            root: root.as_ref().to_path_buf(),
        })
    }
}
