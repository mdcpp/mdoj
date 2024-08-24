use crate::async_loop;

use self::wrapper::CgroupWrapperOwned;

use super::{stat::*, *};
use cgroups_rs::{cgroup_builder::CgroupBuilder, Cgroup};
use std::sync::{atomic::Ordering, Arc};
use tokio::{select, time::*};

/// maximum allow time deviation for cpu monitor
pub const MONITOR_ACCURACY: Duration = Duration::from_millis(80);

lazy_static::lazy_static! {
    pub static ref CG_PATH_COUNTER: AtomicUsize=AtomicUsize::new(0);
}

async fn monitor(cgroup: Arc<Cgroup>, cpu: Cpu) -> MonitorKind {
    let wrapper = wrapper::CgroupWrapper::new(&cgroup);

    let oom_signal = wrapper.oom_signal();

    let cpu_future = async_loop!({
        if Cpu::out_of_resources(&cpu, wrapper.cpu()) {
            break;
        }
    });

    select! {
        _ = cpu_future=> MonitorKind::Cpu,
        _ = oom_signal.wait()=> MonitorKind::Memory
    }
}

/// monitor resource of cpu and memory
pub struct Monitor {
    cgroup: Arc<Cgroup>,
    cpu: Cpu,
    monitor_task: Option<tokio::task::JoinHandle<MonitorKind>>,
}

impl Drop for Monitor {
    fn drop(&mut self) {
        if let Some(monitor_task) = &self.monitor_task {
            monitor_task.abort();
        }
        // FIXME: use explicit control flow
        // currently is controlled by dropping order, and it can be broken
        // if one of the thread panics
        match self.cgroup.v2() {
            true => {
                self.cgroup.kill().expect("cgroup.kill does not exist");
                CgroupWrapperOwned::new(&self.cgroup).poll_delete();
            }
            false => {
                self.cgroup.set_release_agent("").unwrap();
            }
        }
    }
}

impl Monitor {
    /// create a new limiter and mount at given path
    pub fn new((mem, cpu): MemAndCpu) -> Result<Self, Error> {
        let cg_name = format!("mdoj.{}", CG_PATH_COUNTER.fetch_add(1, Ordering::AcqRel));
        log::trace!("create cgroup, name: {}", cg_name);
        let cgroup = Arc::new(
            CgroupBuilder::new(&cg_name)
                .memory()
                .kernel_memory_limit(mem.kernel as i64)
                .memory_hard_limit(mem.user as i64)
                .memory_swap_limit(0)
                .done()
                .cpu()
                // .period((MONITOR_ACCURACY / 2).as_nanos() as u64)
                .quota(MONITOR_ACCURACY.as_nanos() as i64)
                .realtime_period(MONITOR_ACCURACY.as_nanos() as u64)
                // .realtime_runtime(MONITOR_ACCURACY.as_nanos() as i64)
                .done()
                // .set_specified_controllers(vec!["cpu","memory","pids"].into_iter().map(|x|x.to_string()).collect())
                .build(MONITER_KIND.heir())?,
        );
        // FIXME: set oom control

        let monitor_task = Some(tokio::spawn(monitor(cgroup.clone(), cpu.clone())));

        log::debug!("cgroup created: {}", cgroup.path());
        Ok(Self {
            cgroup,
            monitor_task,
            cpu,
        })
    }
    pub fn get_cg_path(&self) -> &str {
        self.cgroup.path()
    }
}

impl super::Monitor for Monitor {
    type Resource = MemAndCpu;
    /// wait for resource to exhaust
    ///
    /// Please remember that [`Drop::drop`] only optimistic kill(`SIGKILL`)
    /// the process inside it,
    /// user SHOULD NOT rely on this to kill the process.
    ///
    ///
    /// 2. Actively limit(notify) cpu resource is achieved by polling the cgroup,
    ///     the delay require special attention, it is only guaranteed
    ///     to below limitation provided + [`MONITOR_ACCURACY`].
    ///
    /// This method is cancellation safe
    async fn wait_exhaust(&mut self) -> MonitorKind {
        self.monitor_task.as_mut().unwrap().await.unwrap()
    }
    fn poll_exhaust(&mut self) -> Option<MonitorKind> {
        let wrapper = wrapper::CgroupWrapper::new(&self.cgroup);

        if wrapper.oom() {
            return Some(MonitorKind::Memory);
        } else if Cpu::out_of_resources(&self.cpu, wrapper.cpu()) {
            return Some(MonitorKind::Cpu);
        }
        None
    }
    /// get the final resource usage
    ///
    /// Please remember thatActively limit(notify) cpu resource is achieved
    /// by polling the cgroup, therefore the delay require special attention,
    /// it is only guaranteed to below limitation provided + [`MONITOR_ACCURACY`].
    async fn stat(self) -> Self::Resource {
        // FIXME: check running process, this line is commented out because of uncollected process
        // uncollected process is at state of not running, but pid is still in use
        // debug_assert!(self.cgroup.tasks().is_empty());
        let wrapper = wrapper::CgroupWrapper::new(&self.cgroup);
        (wrapper.memory(), wrapper.cpu())
    }
}

// FIXME: mock cgroup and test it
