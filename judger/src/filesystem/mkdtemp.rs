use std::{
    ffi::{CStr, CString, OsStr},
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
};

use tokio::fs::remove_dir;

pub struct MkdTemp(PathBuf);

impl Drop for MkdTemp {
    fn drop(&mut self) {
        tokio::spawn(remove_dir(self.0.clone()));
    }
}

impl MkdTemp {
    pub fn new() -> Self {
        Self(unsafe { Self::new_inner("mdoj-XXXXXX") })
    }
    pub unsafe fn new_inner(template: &str) -> PathBuf {
        let template = CString::new(template).unwrap();
        let tmp_ptr = libc::mkdtemp(template.as_ptr() as *mut _);
        let tmp_path = CStr::from_ptr(tmp_ptr);
        let str_path = OsStr::from_bytes(tmp_path.to_bytes());
        drop(template);
        // libc::free(tmp_ptr as *mut _);
        PathBuf::from(str_path)
    }
    pub fn get_path(&self) -> &Path {
        self.0.as_path()
    }
}
