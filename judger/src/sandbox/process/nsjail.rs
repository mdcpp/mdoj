use std::{
    borrow::Cow,
    ffi::{OsStr, OsString},
    ops::Deref,
    os::unix::ffi::OsStrExt,
    path::Path,
};

use crate::CONFIG;

pub static NSJAIL_PATH: &str = "./nsjail-3.1";

pub trait Argument {
    fn get_args(self) -> impl Iterator<Item = Cow<'static, OsStr>>;
}

/// factory pattern for combining arguments
#[derive(Default)]
pub struct ArgFactory {
    args: Vec<Cow<'static, OsStr>>,
}

impl ArgFactory {
    pub fn add(mut self, arg: impl Argument) -> Self {
        self.args.extend(arg.get_args());
        self
    }

    pub fn build(self) -> Vec<OsString> {
        self.args.into_iter().map(|x| x.into_owned()).collect()
    }
}

/// base arguments for nsjail
pub struct BaseArg;

impl Argument for BaseArg {
    fn get_args(self) -> impl Iterator<Item = Cow<'static, OsStr>> {
        let mut args = vec![
            // FIXME: MODE_STANDALONE_ONCE would might cause sandbox to continue running after process exit, check if that's true
            Cow::Borrowed(OsStr::from_bytes(b"-Mo")),
            Cow::Borrowed(OsStr::from_bytes(b"-l")),
            #[cfg(not(debug_assertions))]
            Cow::Borrowed(OsStr::from_bytes(b"/dev/null")),
            #[cfg(debug_assertions)]
            Cow::Borrowed(OsStr::from_bytes(b"nsjail.log")),
            Cow::Borrowed(OsStr::from_bytes(b"--env")),
            Cow::Borrowed(OsStr::from_bytes(
                b"PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
            )),
        ];

        if !CONFIG.rootless {
            log::debug!("running in root mode");
            args.push(Cow::Borrowed(OsStr::from_bytes(b"--disable_clone_newuser")));
        }
        args.into_iter()
    }
}

/// arguments for setting cgroup
pub struct CGroupMountArg<'a> {
    pub cg_name: &'a str,
}

impl<'a> Argument for CGroupMountArg<'a> {
    fn get_args(self) -> impl Iterator<Item = Cow<'static, OsStr>> {
        // note that there is cg_name, cg_path and cg_mount, they are different!
        match super::monitor::CGROUP_V2.deref() {
            // this is a patch(not default behavior of nsjail)
            true => vec![
                Cow::Borrowed(OsStr::from_bytes(b"--disable_clone_newcgroup")),
                Cow::Borrowed(OsStr::from_bytes(b"--cgroup_mem_swap_max")),
                Cow::Borrowed(OsStr::from_bytes(b"0")),
                Cow::Borrowed(OsStr::from_bytes(b"--cgroup_cpu_parent")),
                Cow::Owned(OsString::from(self.cg_name)),
            ],
            false => vec![
                Cow::Borrowed(OsStr::from_bytes(b"--disable_clone_newcgroup")),
                Cow::Borrowed(OsStr::from_bytes(b"--cgroup_mem_swap_max")),
                Cow::Borrowed(OsStr::from_bytes(b"0")),
                Cow::Borrowed(OsStr::from_bytes(b"--cgroup_mem_mount")),
                Cow::Owned(format!("/sys/fs/cgroup/memory/{}", self.cg_name).into()),
                Cow::Borrowed(OsStr::from_bytes(b"--cgroup_cpu_mount")),
                Cow::Owned(format!("/sys/fs/cgroup/cpu/{}", self.cg_name).into()),
                Cow::Borrowed(OsStr::from_bytes(b"--cgroup_pids_mount")),
                Cow::Owned(format!("/sys/fs/cgroup/pids/{}", self.cg_name).into()),
            ],
        }
        .into_iter()
    }
}

/// arguments for setting cgroup version
pub struct CGroupVersionArg;

impl Argument for CGroupVersionArg {
    fn get_args(self) -> impl Iterator<Item = Cow<'static, OsStr>> {
        match super::monitor::CGROUP_V2.deref() {
            true => vec![Cow::Borrowed(OsStr::from_bytes(b"--use_cgroupv2"))],
            false => Vec::new(),
        }
        .into_iter()
    }
}

/// arguments for rootfs mount
pub struct MountArg<'a> {
    pub rootfs: &'a Path,
}

impl<'a> Argument for MountArg<'a> {
    fn get_args(self) -> impl Iterator<Item = Cow<'static, OsStr>> {
        vec![
            Cow::Borrowed(OsStr::from_bytes(b"--tmpfsmount")),
            Cow::Borrowed(OsStr::from_bytes(b"/tmp")),
            Cow::Borrowed(OsStr::from_bytes(b"--rw")),
            Cow::Borrowed(OsStr::from_bytes(b"--chroot")),
            Cow::Owned(OsString::from(self.rootfs)),
        ]
        .into_iter()
    }
}

/// arguments for launching inner process
pub struct InnerProcessArg<'a, I>
where
    I: Iterator<Item = &'a OsStr>,
{
    pub inner_args: I,
}

impl<'a, I> Argument for InnerProcessArg<'a, I>
where
    I: Iterator<Item = &'a OsStr>,
{
    fn get_args(self) -> impl Iterator<Item = Cow<'static, OsStr>> {
        vec![Cow::Borrowed(OsStr::from_bytes(b"--"))]
            .into_iter()
            .chain(self.inner_args.map(|x| Cow::Owned(x.to_owned())))
    }
}
