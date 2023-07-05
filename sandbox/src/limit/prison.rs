use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicI64, Ordering},
};

use tokio::fs;

use crate::init::config::CONFIG;

use super::{unit::Unit, utils::preserve::MemoryCounter, Error};

pub struct Prison {
    id_counter: AtomicI64,
    pub(super) memory_counter: MemoryCounter,
    pub(super) tmp: PathBuf,
}

impl Prison {
    pub fn new(tmp: impl AsRef<Path>) -> Self {
        let config = CONFIG.get().unwrap();
        Self {
            id_counter: Default::default(),
            memory_counter: MemoryCounter::new(config.platform.available_memory),
            tmp: tmp.as_ref().to_path_buf(),
        }
    }
    // pub fn usage(&self) -> ResourceUsage {
    //     self.resource.usage()
    // }
    pub async fn create<'a>(&'a self, root: impl AsRef<Path>) -> Result<Unit<'a>, Error> {
        let id = self.id_counter.fetch_add(1, Ordering::Release).to_string();
        let container_root = self.tmp.join(id.clone());

        fs::create_dir(container_root.clone()).await?;
        fs::create_dir(container_root.clone().join("src")).await?;

        Ok(Unit {
            id,
            controller: self,
            root: root.as_ref().to_path_buf(),
        })
    }
}
