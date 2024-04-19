use std::{ffi::OsString, ops::Deref};

#[derive(Default)]
pub struct ArgFactory {
    args: Vec<OsString>,
}

impl ArgFactory {}

trait Argument {
    fn get_args(self) -> Vec<OsString>;
}

struct BaseArg;

impl Argument for BaseArg {
    fn get_args(self) -> Vec<OsString> {
        vec![
            OsString::from("--disable_clone_newuser"),
            OsString::from("--disable_clone_newuser"),
            OsString::from("--disable_clone_newcgroup"),
            OsString::from("--env"),
            OsString::from("PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"),
        ]
    }
}

struct CGroupMountArg {
    pub cg_path: String,
}

impl Argument for CGroupMountArg {
    fn get_args(self) -> Vec<OsString> {
        match super::limiter::CGROUP_V2.deref() {
            true => vec![
                OsString::from("--cgroup_cpu_parent"),
                OsString::from(self.cg_path),
            ],
            false => vec![
                OsString::from("--cgroup_mem_mount"),
                format!("/sys/fs/cgroup/memory/{}", self.cg_path.clone()).into(),
                OsString::from("--cgroup_cpu_mount"),
                format!("/sys/fs/cgroup/cpu/{}", self.cg_path.clone()).into(),
                OsString::from("--cgroup_pids_mount"),
                format!("/sys/fs/cgroup/pids/{}", self.cg_path).into(),
            ]
        }
    }
}

struct CGroupVersionArg;

impl Argument for CGroupVersionArg {
    fn get_args(self) -> Vec<OsString> {
        match super::limiter::CGROUP_V2.deref() {
            true => vec![OsString::from("--use_cgroupv2")],
            false => Vec::new(),
        }
    }
}
