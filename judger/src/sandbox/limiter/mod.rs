//! Provide ability to limit resource and retrieve final cpu and memory usage
//!
//! To use this module, you need to create it (provide resource limitation) and mount it,
//! finally, spawn process(it's user's responsibility to ensure the process
//! is spawned within the cgroup)
pub(self) mod hier;
pub(self) mod stat;
pub(self) mod wrapper;

pub use stat::*;

use crate::error::Error;
use cgroups_rs::{cgroup_builder::CgroupBuilder, *};
use hier::*;
use std::sync::Arc;
use tokio::time::*;

const MONITOR_ACCURACY: Duration = Duration::from_millis(80);

lazy_static::lazy_static! {
    pub static ref CGROUP_V2:bool=hier::MONITER_KIND.heir().v2();
}

/// Exit reason of the process
pub enum ExitReason {
    TimeOut,
    MemoryOut,
}

pub async fn monitor(cgroup: Arc<Cgroup>, cpu: Cpu) -> ExitReason {
    let wrapper = wrapper::CgroupWrapper::new(&cgroup);

    let oom_signal = wrapper.oom();

    loop {
        sleep(MONITOR_ACCURACY / 2).await;

        if let Ok(oom_hint) = oom_signal.try_recv() {
            log::trace!("oom hint: {}", oom_hint);
            return ExitReason::MemoryOut;
        }

        if Cpu::out_of_resources(&cpu, wrapper.cpu()) {
            return ExitReason::TimeOut;
        }
        wrapper.cpu();
    }
}

pub struct Limiter {
    cgroup: Arc<Cgroup>,
    monitor_task: Option<tokio::task::JoinHandle<ExitReason>>,
}

impl Drop for Limiter {
    fn drop(&mut self) {
        if let Some(monitor_task) = &self.monitor_task {
            monitor_task.abort();
        }
        debug_assert!(self.cgroup.tasks().is_empty());
        match self.cgroup.tasks().is_empty() {
            true => log::warn!("cgroup still have process running"),
            false => self.cgroup.delete().expect("cgroup cannot be deleted"),
        }
    }
}

impl Limiter {
    /// create a new limiter and mount at given path
    pub fn new_mount(cg_path: &str, cpu: Cpu, mem: Memory) -> Result<Self, Error> {
        let cgroup = Arc::new(
            CgroupBuilder::new(cg_path)
                .memory()
                .kernel_memory_limit(mem.kernel as i64)
                .memory_hard_limit(mem.user as i64)
                .memory_swap_limit(0)
                .done()
                .cpu()
                .period((MONITOR_ACCURACY / 2).as_nanos() as u64)
                .quota(MONITOR_ACCURACY.as_nanos() as i64)
                .realtime_period(MONITOR_ACCURACY.as_nanos() as u64)
                .realtime_runtime(MONITOR_ACCURACY.as_nanos() as i64)
                .done()
                .build(MONITER_KIND.heir())?,
        );

        let monitor_task = Some(tokio::spawn(monitor(cgroup.clone(), cpu)));

        Ok(Self {
            cgroup,
            monitor_task,
        })
    }
    /// wait for resource to exhaust
    ///
    /// Please remember that [`Drop::drop`] only optimistic kill(`SIGKILL`)
    /// the process inside it,
    /// user SHOULD NOT rely on this to kill the process.
    ///
    ///
    /// 2. Actively limit(notify) cpu resource is achieved by polling the cgroup,
    /// the delay require special attention, it is only guaranteed
    /// to below limitation provided + [`MONITOR_ACCURACY`].
    pub async fn wait_exhaust(&mut self) -> ExitReason {
        let reason = self.monitor_task.take().unwrap().await.unwrap();
        // optimistic kill(`SIGKILL`) the process inside
        self.cgroup.kill().expect("cgroup.kill does not exist");
        reason
    }
    /// get the final resource usage
    ///
    /// Please remember thatActively limit(notify) cpu resource is achieved
    /// by polling the cgroup, therefore the delay requirespecial attention,
    /// it is only guaranteed to below limitation provided + [`MONITOR_ACCURACY`].
    pub async fn stat(self) -> (Cpu, Memory) {
        // there should be no process left
        debug_assert!(self.cgroup.tasks().is_empty());
        // poll once more to get final stat
        let wrapper = wrapper::CgroupWrapper::new(&self.cgroup);
        (wrapper.cpu(), wrapper.memory())
    }
}
