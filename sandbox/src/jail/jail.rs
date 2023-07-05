use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicI64, Ordering},
};

use super::{
    limit::LimitReason,
    resource::{ResourceCounter, ResourceGuard, ResourceUsage},
    Error,
};
use std::process::Stdio;
use tokio::{
    fs,
    io::{AsyncReadExt, AsyncWriteExt},
    process::{Child, ChildStderr, ChildStdin, ChildStdout, Command},
    select, time,
};

use crate::{
    init::config::CONFIG,
    jail::limit::{CpuLimit, Limiter, MemLimit},
};

const NICE: i32 = 18;

pub struct Limit {
    pub lockdown: bool,
    pub cpu_us: u64,
    pub rt_us: i64,
    pub total_us: u64,
    pub user_mem: i64,
    pub kernel_mem: i64,
    pub swap_user: i64,
}

impl Limit {
    pub fn max() -> Self {
        Self {
            lockdown: true,
            cpu_us: u64::MAX / 2 - 1,
            rt_us: i64::MAX / 2 - 1,
            total_us: u64::MAX / 2 - 1,
            user_mem: i64::MAX / 2 - 1,
            kernel_mem: i64::MAX / 2 - 1,
            swap_user: 0,
        }
    }
}

pub struct Prison {
    id_counter: AtomicI64,
    resource: ResourceCounter,
    tmp: PathBuf,
}

impl Prison {
    pub fn new(tmp: impl AsRef<Path>) -> Self {
        let config = CONFIG.get().unwrap();
        Self {
            id_counter: Default::default(),
            resource: ResourceCounter::new(config.platform.available_memory),
            tmp: tmp.as_ref().to_path_buf(),
        }
    }
    pub fn usage(&self) -> ResourceUsage {
        self.resource.usage()
    }
    pub async fn create<'a>(&'a self, root: impl AsRef<Path>) -> Result<Cell<'a>, Error> {
        let id = self.id_counter.fetch_add(1, Ordering::Release).to_string();
        let container_root = self.tmp.join(id.clone());

        fs::create_dir(container_root.clone()).await?;
        fs::create_dir(container_root.clone().join("src")).await?;

        Ok(Cell {
            id,
            controller: self,
            root: root.as_ref().to_path_buf(),
        })
    }
}

pub struct Cell<'a> {
    id: String,
    controller: &'a Prison,
    root: PathBuf,
}

impl<'a> Drop for Cell<'a> {
    fn drop(&mut self) {
        let tmp_path = self.controller.tmp.as_path().clone().join(self.id.clone());
        tokio::spawn(async {
            log::trace!("removing Cell's presistent storage");
            fs::remove_dir_all(tmp_path).await
        });
    }
}

impl<'a> Cell<'a> {
    pub async fn execute(&self, args: &Vec<&str>, limit: Limit) -> Result<Process, Error> {
        log::debug!("preparing Cell");
        let config = CONFIG.get().unwrap();

        let cgroup_name = format!("mdoj.{}", self.id);

        let reversed_memory = limit.user_mem + limit.kernel_mem;

        let resource_guard = self.controller.resource.allocate(reversed_memory).await?;

        let mem_limit: MemLimit = MemLimit {
            user: limit.user_mem,
            kernel: limit.kernel_mem,
            swap: limit.swap_user,
        };
        let cpu_limit: CpuLimit = CpuLimit {
            cpu_us: limit.cpu_us,
            rt_us: limit.rt_us,
            total_us: limit.total_us,
        };

        let mut cmd: Vec<&str> = Vec::new();

        cmd.push(&config.nsjail.runtime);

        let presistent_volume = self.controller.tmp.as_path().clone().join(self.id.clone());
        let presistent_volume = presistent_volume.join("src").canonicalize().unwrap();
        let mut presistent_volume = presistent_volume.to_str().unwrap().to_owned();
        presistent_volume.push_str(":/src");

        if limit.lockdown {
            cmd.push("--bindmount_ro");
        } else {
            cmd.push("--bindmount");
        }
        cmd.push(&presistent_volume);

        if !config.nsjail.rootless {
            cmd.push("--disable_clone_newuser");
        }

        cmd.push("-m none:tmp:tmpfs:size=268435456");
        cmd.push("--use_cgroupv2");
        cmd.push("--disable_clone_newcgroup");

        cmd.push("--cgroup_mem_swap_max");
        cmd.push("0");

        cmd.push("--cgroupv2_mount");
        let cgroup_mount = format!("/sys/fs/cgroup/{}", &cgroup_name);
        cmd.push(&cgroup_mount);

        cmd.push("-Me");

        cmd.push("-l");

        let nsjail_log = Path::new(&config.nsjail.log).canonicalize().unwrap();
        cmd.push(nsjail_log.to_str().unwrap());

        let chroot = self.root.canonicalize().unwrap();
        cmd.push("--chroot");
        cmd.push(chroot.to_str().unwrap());

        let nice = NICE.to_string();
        cmd.push("--nice_level");
        cmd.push(&nice);

        cmd.push("--");

        for arg in args {
            cmd.push(arg);
        }

        log::debug!("Starting Cell: {}", cmd.join(" "));

        let mut fcmd: Command = Command::new(cmd[0]);

        for i in 1..cmd.len() {
            fcmd.arg(cmd[i]);
        }
        let process = fcmd
            .stdout(Stdio::piped())
            .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let limiter = Limiter::from_limit(&cgroup_name, cpu_limit, mem_limit, process.id())?;

        let process = Some(process);

        Ok(Process {
            process,
            limiter,
            resource_guard,
        })
    }
}

pub struct Process<'a> {
    process: Option<Child>,
    limiter: Limiter,
    resource_guard: ResourceGuard<'a>,
}

#[derive(PartialEq, Eq, Debug)]
pub enum ProcessStatus {
    Exit(i32),
    Exhaust(LimitReason),
    SigExit,
    Stall,
}

impl ProcessStatus {
    pub fn succeed(&self) -> bool {
        match self {
            ProcessStatus::Exit(x) => *x == 0,
            _ => false,
        }
    }
}

impl<'a> Drop for Process<'a> {
    fn drop(&mut self) {
        let mut process = self.process.take().unwrap();
        tokio::spawn(async move {
            process.kill().await.unwrap();
            process.wait().await.unwrap();
        });
    }
}

impl<'a> Process<'a> {
    pub async fn kill(&mut self) {
        self.process.take().unwrap().kill().await.ok();
    }
    pub fn stdin(&mut self) -> Option<ChildStdin> {
        self.process.as_mut().unwrap().stdin.take()
    }
    pub fn stdout(&mut self) -> Option<ChildStdout> {
        self.process.as_mut().unwrap().stdout.take()
    }
    pub fn stderr(&mut self) -> Option<ChildStderr> {
        self.process.as_mut().unwrap().stderr.take()
    }
    pub async fn wait(&mut self) -> ProcessStatus {
        select! {
            x = self.process.as_mut().unwrap().wait() => {
                match x.unwrap().code() {
                    Some(x) => ProcessStatus::Exit(x),
                    None => ProcessStatus::SigExit
                }
            }
            _ = self.limiter.wait() => {
                ProcessStatus::Exhaust(self.limiter.status().unwrap())
            }
            _ = time::sleep(time::Duration::from_secs(3600)) => {
                ProcessStatus::Stall
            }
        }
    }
    pub async fn write_all(&mut self, buf: Vec<u8>) -> Result<(), Error> {
        self.process
            .as_mut()
            .unwrap()
            .stdin
            .as_mut()
            .ok_or(Error::CapturedPiped)?
            .write_all(&buf)
            .await?;
        Ok(())
    }
    pub async fn read_all(&mut self) -> Result<Vec<u8>, Error> {
        let mut buf = Vec::new();
        self.process
            .as_mut()
            .unwrap()
            .stdout
            .as_mut()
            .ok_or(Error::CapturedPiped)?
            .read_to_end(&mut buf)
            .await?;
        Ok(buf)
    }
    pub fn cpu_usage(&mut self) -> CpuLimit {
        self.limiter.cpu_usage()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn exec() {
        crate::init::new().await;

        {
            let prison = Prison::new(".temp");
            let cell = prison.create("plugins/lua-5.2/rootfs").await.unwrap();

            let mut process = cell
                .execute(
                    &vec!["/usr/local/bin/lua", "/test.lua"],
                    Limit {
                        cpu_us: 1000 * 1000 * 1000,
                        rt_us: 1000 * 1000 * 1000,
                        total_us: 30 * 1000,
                        swap_user: 0,
                        kernel_mem: 128 * 1024 * 1024,
                        user_mem: 512 * 1024 * 1024,
                        lockdown: false,
                    },
                )
                .await
                .unwrap();

            let status = process.wait().await;

            assert!(status.succeed());

            let out = process.read_all().await.unwrap();
            assert_eq!(out, b"hello world\n");
        }

        // unlike async-std, tokio won't wait for all background task to finish before exit
        time::sleep(time::Duration::from_millis(12)).await;
    }
}
