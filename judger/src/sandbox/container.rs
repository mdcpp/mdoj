use std::{
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

use tokio::fs;

use crate::{
    init::config::CONFIG,
    sandbox::utils::{limiter::Limiter, nsjail::NsJail},
};

use super::{daemon::ContainerDaemon, process::RunningProc, Error, Limit};

/// cgroup counter
///
/// The first container would be mount at /sys/fs/cgroup/mdoj/0
static CG_COUNTER: AtomicUsize = AtomicUsize::new(0);

// Container abstraction, call nsjail to execute process, limiter to limit resources
// expect downstream(daemon) setup up and clear tmp files
pub struct Container<'a> {
    pub(super) id: String,
    pub(super) daemon: &'a ContainerDaemon,
    pub(super) root: PathBuf,
}

impl<'a> Drop for Container<'a> {
    fn drop(&mut self) {
        let tmp_path = self.daemon.tmp.as_path().join(self.id.clone());
        log::trace!("Cleaning up container with id :{}", self.id);
        tokio::spawn(async { fs::remove_dir_all(tmp_path).await });
    }
}

impl<'a> Container<'a> {
    pub async fn execute(&self, args: &[&str], limit: Limit) -> Result<RunningProc, Error> {
        let config = CONFIG.get().unwrap();

        log::trace!("Preparing container with id :{} for new process", self.id);

        let cg_name = format!(
            "{}{}",
            config.runtime.root_cgroup,
            CG_COUNTER.fetch_add(1, Ordering::Acquire)
        );

        let reversed_memory = limit.user_mem + limit.kernel_mem;
        let output_limit = config.platform.output_limit as u64;

        let memory_holder = self
            .daemon
            .memory_counter
            .allocate(output_limit + reversed_memory)
            .await?;

        let nsjail = NsJail::builder(&self.root)
            .cgroup(&cg_name)
            .done()
            .presist_vol(&self.id)
            .mount("src", false)
            .done()
            .common()
            .cmds(args)
            .build()?;

        let limiter = Limiter::new(&cg_name, limit)?;

        Ok(RunningProc {
            limiter,
            nsjail,
            _memory_holder: memory_holder,
        })
    }
}
