use std::{
    borrow::Cow,
    path::{Path, PathBuf},
    process::Stdio, fmt::format,
};

use tokio::{
    process::{Child, Command},
    sync::Mutex,
};

use crate::init::config::CONFIG;

use super::super::Error;

pub struct LimitBuilder {
    cmds: Vec<Cow<'static, str>>,
}

impl LimitBuilder {
    pub fn cgroup(mut self, cgroup_name: &str) -> LimitBuilder {
        let config=CONFIG.get().unwrap();
        match config.nsjail.is_cgv1(){
            true => {
                self.cmds.push(Cow::Borrowed("--cgroup_mem_parent"));
                self.cmds.push(Cow::Owned(cgroup_name.to_owned()));
                self.cmds.push(Cow::Borrowed("--cgroup_cpu_parent"));
                self.cmds.push(Cow::Owned(cgroup_name.to_owned()));
                self.cmds.push(Cow::Borrowed("--cgroup_cpu_ms_per_sec"));
                self.cmds.push(Cow::Borrowed("1000000000000"));
            },
            false => {
                self.cmds.push(Cow::Borrowed("--use_cgroupv2"));
                self.cmds.push(Cow::Borrowed("--cgroup_cpu_parent"));
                self.cmds.push(Cow::Owned(cgroup_name.to_owned()));
            },
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

        self.cmds.push(Cow::Owned(format!("{}:{}", source, dist)));

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
            .stdout(Stdio::piped())
            .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        Ok(NsJail {
            process: Some(Mutex::new(process)),
        })
    }
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
    pub fn new(root: impl AsRef<Path>) -> LimitBuilder {
        let root = root.as_ref().canonicalize().unwrap();
        let root = root.to_str().unwrap();
        let mut cmds: Vec<Cow<'static, str>> = Vec::new();
        cmds.push(Cow::Borrowed("--chroot"));
        cmds.push(Cow::Owned(root.to_owned()));
        LimitBuilder { cmds }
    }
    pub async fn wait(&self) -> Option<i32> {
        let status = self
            .process
            .as_ref()
            .unwrap()
            .lock()
            .await
            .wait()
            .await
            .unwrap();
        status.code()
    }
    pub async fn kill(&self) -> Result<(), Error> {
        self.process.as_ref().unwrap().lock().await.kill().await?;
        Ok(())
    }
}
