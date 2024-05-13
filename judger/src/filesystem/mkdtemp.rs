use std::{
    ffi::{CString, OsStr},
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
};

use tokio::fs::remove_dir;

/// A safe wrapper around [`libc::mkdtemp`]
pub struct MkdTemp(PathBuf);

impl Drop for MkdTemp {
    fn drop(&mut self) {
        tokio::spawn(remove_dir(self.0.clone()));
    }
}

impl MkdTemp {
    /// Create a new MkdTemp
    pub fn new() -> Self {
        Self(unsafe { Self::new_inner("/tmp/mdoj-fs-runtime-XXXXXX") })
    }
    pub unsafe fn new_inner(template: &str) -> PathBuf {
        let template = CString::new(template).unwrap();
        libc::mkdtemp(template.as_ptr() as *mut _);
        let str_path = OsStr::from_bytes(template.to_bytes());
        PathBuf::from(str_path)
    }
    /// get_path acquired by the MkdTemp
    pub fn get_path(&self) -> &Path {
        self.0.as_path()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::time::sleep;

    use super::*;

    #[tokio::test]
    async fn test_mkdtemp() {
        let tmp = MkdTemp::new();
        let path = tmp.get_path().to_path_buf();
        println!("{:?}", path);
        drop(tmp);
        sleep(Duration::from_millis(20)).await;
        assert!(!path.exists());
    }
}
