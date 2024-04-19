pub(self) mod hier;
pub(self) mod stat;
pub(self) mod wrapper;

pub use stat::*;

use crate::error::Error;
use cgroups_rs::{cgroup_builder::CgroupBuilder, *};
use hier::*;
use std::sync::Arc;
use tokio::time::*;

pub enum ExitReason {
    TimeOut,
    MemoryOut,
}

pub async fn monitor(cgroup: Arc<Cgroup>, cpu: Cpu) -> ExitReason {
    let wrapper = wrapper::CgroupWrapper::new(&cgroup);

    let oom_signal = wrapper.oom();

    loop {
        sleep(MONITOR_ACCURACY/2).await;

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

/// limiter that monitor the resource usage of a cgroup
pub struct Limiter {
    cgroup: Arc<Cgroup>,
    monitor_task: Option<tokio::task::JoinHandle<ExitReason>>,
}

impl Drop for Limiter {
    fn drop(&mut self) {
        if let Some(monitor_task) = &self.monitor_task {
            monitor_task.abort();
        }
        if MONITER_KIND.heir().v2() {
            self.cgroup.kill().expect("cgroup.kill does not exist");
        } else {
            // use rustix::process::*;
            // pid should not be reused until SIGPIPE send(when Process is Drop)
            // therefore, it is safe to try killing the process(only true for nsjail)

            // current implementation of v1 support do nothing, wait for action of cleaning
            // up the process on drop
            // for pid in self.cgroup.tasks() {}
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
                .period((MONITOR_ACCURACY/2).as_nanos() as u64)
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
    pub async fn wait_exhaust(&mut self) -> ExitReason {
        self.monitor_task.take().unwrap().await.unwrap()
    }
    /// get the current resource usage
    pub async fn stat(self)->(Cpu,Memory){
        let wrapper = wrapper::CgroupWrapper::new(&self.cgroup);
        (wrapper.cpu(),wrapper.memory())
    }
}
