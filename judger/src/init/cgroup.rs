use std::{fs, os::unix::prelude::OsStrExt, path::Path};

use super::config::CONFIG;

// Clean up cgroup
pub fn init() {
    let config = CONFIG.get().unwrap();
    let root_cg = Path::new("/sys/fs/cgroup").join(config.runtime.root_cgroup.clone());
    if root_cg.exists() {
        for sub_cgroup in root_cg.read_dir().unwrap().flatten() {
            if let Ok(meta) = sub_cgroup.metadata() {
                if meta.is_dir() {
                    let mut path = sub_cgroup.path();
                    remove_nsjail(&mut path);
                    fs::remove_dir(path).unwrap();
                }
            }
        }
    }
}

pub fn remove_nsjail(path: &mut Path) {
    log::debug!("Cleaning up cgroup in {}", path.to_string_lossy());
    if let Ok(rd) = path.read_dir() {
        for nsjail in rd.flatten() {
            if nsjail.file_name().as_bytes().starts_with(b"NSJAIL") {
                fs::remove_dir(nsjail.path()).unwrap();
            }
        }
    }
}
