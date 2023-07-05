use std::{
    path::{Path, PathBuf},
    process::Stdio,
    sync::Arc,
};

use tokio::{
    process::{Child, Command},
    sync::Mutex,
};

use crate::init::config::CONFIG;

use super::super::Error;

pub struct LimitBuilder {
    cmds: Vec<String>,
}

impl LimitBuilder {
    pub fn cgroup(mut self, cgroup_name: &str) -> LimitBuilder {
        self.cmds.push("--use_cgroupv2".to_owned());

        let cgroup_mount = format!("/sys/fs/cgroup/{}", &cgroup_name);
        self.cmds.push("--cgroupv2_mount".to_owned());
        self.cmds.push(cgroup_mount);

        self
    }
    pub fn done(mut self) -> NaJailBuilder {
        self.cmds.push("--disable_clone_newcgroup".to_owned());
        self.cmds.push("--cgroup_mem_swap_max".to_owned());
        self.cmds.push("0".to_owned());
        self.cmds.push("--disable_clone_newcgroup".to_owned());
        self.cmds.push("--disable_clone_newcgroup".to_owned());
        NaJailBuilder { cmds: self.cmds }
    }
}

pub struct NaJailBuilder {
    cmds: Vec<String>,
}

impl NaJailBuilder {
    pub fn presist_vol(self, id: &str) -> MountBuilder {
        let config = CONFIG.get().unwrap();
        let presist_vol = config
            .runtime
            .temp2
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
    cmds: Vec<String>,
}

impl MountBuilder {
    pub fn mount(mut self, vol: &str, lockdown: bool) -> MountBuilder {
        if lockdown {
            self.cmds.push("--bindmount_ro".to_owned());
        } else {
            self.cmds.push("--bindmount".to_owned());
        }

        let source = self.presist_vol.join(vol);
        let source = source.to_str().unwrap();
        let dist = vol;

        self.cmds.push(format!("{}:{}", source, dist));

        self
    }
    pub fn done(mut self) -> CommonBuilder {
        self.cmds.push("--".to_owned());
        CommonBuilder { cmds: self.cmds }
    }
}

pub struct CommonBuilder {
    cmds: Vec<String>,
}

impl CommonBuilder {
    pub fn common(mut self) -> CmdBuilder {
        let config = CONFIG.get().unwrap();

        self.cmds.push("-Me".to_owned());
        self.cmds.push("-l".to_owned());

        self.cmds.push(config.nsjail.log.to_owned());

        self.cmds.push("--".to_owned());

        CmdBuilder { cmds: self.cmds }
    }
}

pub struct CmdBuilder {
    cmds: Vec<String>,
}

impl CmdBuilder {
    pub fn cmds(&mut self, cmd: String) {
        self.cmds.push(cmd);
    }
    pub fn build(self) -> Result<NsJail, Error> {
        let config = CONFIG.get().unwrap();
        let mut cmd: Command = Command::new(&config.nsjail.runtime);

        cmd.args(self.cmds);

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
        let cmds: Vec<String> = vec!["--chroot".to_owned(), root.to_owned()];
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
