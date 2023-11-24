use std::{
    borrow::Cow,
    os::unix::process::ExitStatusExt,
    path::{Path, PathBuf},
    process::Stdio,
};

use tokio::{
    process::{Child, Command},
    sync::Mutex,
};

use crate::init::config::CONFIG;

use super::super::Error;

// Nsjail abstraction, don't implement any meaningful logic
// Just a wrapper for nsjail

pub struct LimitBuilder {
    cmds: Vec<Cow<'static, str>>,
}

impl LimitBuilder {
    pub fn cgroup(mut self, cgroup_name: &str) -> LimitBuilder {
        let config = CONFIG.get().unwrap();
        match config.nsjail.is_cgv1() {
            true => {
                self.cmds.push(Cow::Borrowed("--cgroup_mem_parent"));
                self.cmds.push(Cow::Owned(cgroup_name.to_owned()));
                self.cmds.push(Cow::Borrowed("--cgroup_cpu_parent"));
                self.cmds.push(Cow::Owned(cgroup_name.to_owned()));
                self.cmds.push(Cow::Borrowed("--cgroup_cpu_ms_per_sec"));
                self.cmds.push(Cow::Borrowed("1000000000000"));
            }
            false => {
                self.cmds.push(Cow::Borrowed("--use_cgroupv2"));
                self.cmds.push(Cow::Borrowed("--cgroup_cpu_parent"));
                self.cmds.push(Cow::Owned(cgroup_name.to_owned()));
            }
        }
        // self.cmds.push(Cow::Borrowed("--cgroup_cpu_ms_per_sec"));
        // self.cmds.push(Cow::Borrowed("1"));

        self
    }
    pub fn done(mut self) -> NaJailBuilder {
        self.cmds.push(Cow::Borrowed("--disable_clone_newuser"));
        self.cmds.push(Cow::Borrowed("--cgroup_mem_swap_max"));
        self.cmds.push(Cow::Borrowed("0"));
        self.cmds.push(Cow::Borrowed("--disable_clone_newcgroup"));
        self.cmds.push(Cow::Borrowed("--env"));
        self.cmds.push(Cow::Borrowed(
            "PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
        ));

        NaJailBuilder { cmds: self.cmds }
    }
}

pub struct NaJailBuilder {
    cmds: Vec<Cow<'static, str>>,
}

impl NaJailBuilder {
    pub fn presist_vol(self, id: &str) -> MountBuilder {
        let config = CONFIG.get().unwrap();
        let presist_vol = config
            .runtime
            .temp
            .as_path()
            .join(id)
            .canonicalize()
            .unwrap();

        MountBuilder {
            presist_vol,
            cmds: self.cmds,
        }
    }
}

pub struct MountBuilder {
    presist_vol: PathBuf,
    cmds: Vec<Cow<'static, str>>,
}

impl MountBuilder {
    pub fn mount(mut self, vol: impl AsRef<str>, lockdown: bool) -> MountBuilder {
        if lockdown {
            self.cmds.push(Cow::Borrowed("--bindmount_ro"));
        } else {
            self.cmds.push(Cow::Borrowed("--bindmount"));
        }

        let source = self.presist_vol.join(vol.as_ref());
        let source = source.to_str().unwrap();
        let dist = vol.as_ref();

        self.cmds.push(Cow::Owned(format!("{}:/{}", source, dist)));

        self
    }
    pub fn done(self) -> CommonBuilder {
        CommonBuilder { cmds: self.cmds }
    }
}

pub struct CommonBuilder {
    cmds: Vec<Cow<'static, str>>,
}

impl CommonBuilder {
    pub fn common(mut self) -> CmdBuilder {
        let config = CONFIG.get().unwrap();

        self.cmds.push(Cow::Borrowed("-l"));

        self.cmds.push(Cow::Borrowed(&config.nsjail.log));

        self.cmds.push(Cow::Borrowed("-Me"));

        self.cmds.push(Cow::Borrowed("--"));

        CmdBuilder { cmds: self.cmds }
    }
}

pub struct CmdBuilder {
    cmds: Vec<Cow<'static, str>>,
}

impl CmdBuilder {
    pub fn cmds(mut self, cmd: Vec<&str>) -> NsJailBuilder {
        for arg in cmd.clone() {
            self.cmds.push(Cow::Owned(arg.to_owned()));
        }
        NsJailBuilder { cmds: self.cmds }
    }
}

pub struct NsJailBuilder {
    cmds: Vec<Cow<'static, str>>,
}

impl NsJailBuilder {
    pub fn build(self) -> Result<NsJail, Error> {
        let config = CONFIG.get().unwrap();

        log::trace!(
            "Running subprocess {} {}",
            &config.nsjail.runtime,
            self.cmds.join(" ")
        );

        let mut cmd: Command = Command::new(&config.nsjail.runtime);
        cmd.args(self.cmds.iter().map(|a| a.as_ref()).collect::<Vec<&str>>());

        let process = cmd
            .kill_on_drop(true)
            .stdout(Stdio::piped())
            .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        Ok(NsJail {
            process: Some(Mutex::new(process)),
        })
    }
}

pub enum TermStatus {
    SigExit(i32),
    Code(i32),
}

pub struct NsJail {
    pub process: Option<Mutex<Child>>,
}

impl Drop for NsJail {
    fn drop(&mut self) {
        let process = self.process.take().unwrap();
        tokio::spawn(async move { process.lock().await.kill().await.ok() });
    }
}

impl NsJail {
    pub fn builder(root: impl AsRef<Path>) -> LimitBuilder {
        let root = root.as_ref().canonicalize().unwrap();
        let root = root.to_str().unwrap();
        LimitBuilder {
            cmds: vec![
                Cow::Borrowed("--rw"),
                Cow::Borrowed("--chroot"),
                Cow::Owned(root.to_owned()),
            ],
        }
    }
    pub async fn wait(&self) -> TermStatus {
        let status = self
            .process
            .as_ref()
            .unwrap()
            .lock()
            .await
            .wait()
            .await
            .unwrap();
        if let Some(sig) = status.signal() {
            TermStatus::SigExit(sig)
        } else {
            TermStatus::Code(status.code().unwrap())
        }
    }
}
