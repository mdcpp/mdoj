use crate::sandbox::prelude::*;
use std::path::Path;


use std::sync::Arc;

use cgroups_rs::Cgroup;
use cgroups_rs::{cgroup_builder::CgroupBuilder, hierarchies};
use spin::Mutex;
use tokio::fs;
use tokio::sync::oneshot;
use tokio::sync::oneshot::Receiver;
use tokio::task::JoinHandle;
use tokio::time;

use crate::init::config::CONFIG;

pub mod cpu;
pub mod mem;

/// reason for a process terminate by limiter
pub enum LimitReason {
    Cpu,
    Mem,
    SysMem,
}

/// object for limit resource usage and report it
///
/// resource monitoring start immediately after the initialized
// be aware that cgroup-rs reset all number of the cgroup to zero,
// so limiter should be initialize after `cgroup_rs::Cgroup`
pub struct Limiter {
    task: JoinHandle<()>,
    state: Arc<Mutex<LimiterState>>,
    limit_oneshot: Option<Receiver<LimitReason>>,
    cg_name: String,
    cg: Cgroup,
}

/// state for CpuStatistics, MemStatistics
// why not just make cpu and mem a object and make those own its state?
// because monitoring take time, and we expect cpu and mem not to spawn its own tokio thread
#[derive(Default)]
struct LimiterState {
    cpu: CpuStatistics,
    mem: MemStatistics,
}

impl Drop for Limiter {
    fn drop(&mut self) {
        self.task.abort();
        tokio::spawn(fs::remove_dir(
            Path::new("/sys/fs/cgroup/").join(&self.cg_name),
        ));
    }
}

async fn monitor(
    cg: Cgroup,
    state: Arc<Mutex<LimiterState>>,
    limit: Limit,
    tx: oneshot::Sender<LimitReason>,
) {
    let config = CONFIG.get().unwrap();
    loop {
        time::sleep(time::Duration::from_nanos(config.runtime.accuracy)).await;

        let cpu = CpuStatistics::from_cgroup(&cg);
        let mem = MemStatistics::from_cgroup(&cg);

        // let mut resource_status = ResourceStatus::Running;
        let mut end = false;
        let mut reason = LimitReason::Mem;

        // oom could be incured from invaild configuration
        // check other factor to determine whether is a systm failure or MLE
        if mem.oom {
            log::trace!("Stopping process because it reach its memory limit");
            // even if oom occur, process may still be running(child process killed)
            reason = LimitReason::Mem;
            end = true;
        } else if cpu.rt_us > limit.rt_us
            || cpu.cpu_us > limit.cpu_us
            || cpu.total_us > limit.total_us
        {
            log::trace!("Killing process because it reach its cpu quota");
            reason = LimitReason::Cpu;
            end = true;
        }

        if let Some(mut state) = state.try_lock() {
            state.cpu = cpu;
            state.mem = mem;
        }
        // TODO: use unsafe to increase performance(monitoring is a time critical task)
        // unsafe {
        //     let state_ptr = Box::into_raw(Box::new(LimiterState { cpu, mem }));
        //     drop(Box::from_raw(
        //         state.swap(state_ptr, Ordering::Relaxed),
        //     ));
        // }
        if end {
            tx.send(reason).ok();
            cg.kill().unwrap();
            log::trace!("Process was killed");
            break;
        }
    }
}

impl Limiter {
    /// create limiter with limit
    pub fn new(cg_name: &str, limit: Limit) -> Result<Self, Error> {
        log::trace!("Creating new limiter for {}", cg_name);
        let (tx, rx) = oneshot::channel();

        let state: Arc<Mutex<LimiterState>> = Arc::default();

        let config = CONFIG.get().unwrap();

        let cg = CgroupBuilder::new(cg_name)
            .memory()
            .kernel_memory_limit(limit.kernel_mem as i64)
            .memory_hard_limit(limit.user_mem as i64)
            .memory_swap_limit(limit.swap_user as i64)
            .done()
            .cpu()
            .period(config.runtime.accuracy)
            .quota(config.runtime.accuracy as i64)
            .realtime_period(config.runtime.accuracy)
            .realtime_runtime(config.runtime.accuracy as i64)
            .done();

        let cg = if config.nsjail.is_cgv1() {
            cg.build(Box::new(hierarchies::V1::new()))
        } else {
            cg.build(Box::new(hierarchies::V2::new()))
        }?;

        let cg2 = cg.clone();

        let task = tokio::spawn(monitor(cg.clone(), state.clone(), limit, tx));

        Ok(Limiter {
            task,
            state,
            limit_oneshot: Some(rx),
            cg_name: cg_name.to_owned(),
            cg: cg2,
        })
    }
    /// check if oom
    ///
    /// It expose its internal state(use with care), callee should have explaination for usage
    pub fn check_oom(&mut self) -> bool {
        MemStatistics::from_cgroup(&self.cg).oom
    }
    /// yield statistics, consume self
    pub async fn statistics(self) -> (CpuStatistics, MemStatistics) {
        let config = CONFIG.get().unwrap();

        if !config.kernel.tickless {
            time::sleep(time::Duration::from_nanos(
                (1000 * 1000 / config.kernel.kernel_hz) as u64,
            ))
            .await;
        }
        time::sleep(time::Duration::from_nanos(config.runtime.accuracy)).await;

        let stat = self.state.lock();

        (stat.cpu.clone(), stat.mem.clone())
    }
    /// wait for resouce exhausted
    // it's reverse control flow, subject to change
    pub fn wait_exhausted(&mut self) -> Receiver<LimitReason> {
        self.limit_oneshot
            .take()
            .expect("Limiter cannot be wait twice!")
    }
}
