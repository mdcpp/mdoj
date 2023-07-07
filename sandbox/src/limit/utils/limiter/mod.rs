use crate::limit::Error;
use crate::limit::Limit;
use std::sync::atomic::AtomicPtr;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use cgroups_rs::{cgroup_builder::CgroupBuilder, hierarchies};
use tokio::sync::oneshot;
use tokio::sync::oneshot::Receiver;
use tokio::task::JoinHandle;
use tokio::time;

use crate::init::config::CONFIG;
use crate::limit::utils::limiter::cpu::CpuStatistics;
use crate::limit::utils::limiter::mem::MemStatistics;

pub mod cpu;
pub mod mem;

pub enum LimitReason {
    Cpu,
    Mem,
}

pub struct Limiter {
    task: JoinHandle<()>,
    state: Arc<AtomicPtr<LimiterState>>,
    limit_oneshot: Option<Receiver<LimitReason>>,
}

#[derive(Default)]
struct LimiterState {
    cpu: CpuStatistics,
    mem: MemStatistics,
}

impl Drop for Limiter {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl Limiter {
    pub fn new(cg_name: &str, limit: Limit) -> Result<Self, Error> {
        let (tx, rx) = oneshot::channel();

        let state = Box::into_raw(Box::new(LimiterState::default()));
        let state = Arc::new(AtomicPtr::new(state));

        let config = CONFIG.get().unwrap();

        let hier = Box::new(hierarchies::V2::new());

        let cg = CgroupBuilder::new(&cg_name)
            .memory()
            .kernel_memory_limit(limit.kernel_mem)
            .memory_hard_limit(limit.user_mem)
            .memory_swap_limit(limit.swap_user)
            .done()
            .cpu()
            .period(config.runtime.accuracy)
            .quota(config.runtime.accuracy as i64)
            .realtime_period(config.runtime.accuracy)
            .realtime_runtime(config.runtime.accuracy as i64)
            .done()
            .build(hier)?;

        let state_taken = state.clone();
        let task = tokio::spawn(async move {
            loop {
                time::sleep(time::Duration::from_nanos(config.runtime.accuracy)).await;

                let cpu = CpuStatistics::from_cgroup(&cg);
                let mem = MemStatistics::from_cgroup(&cg);

                // let mut resource_status = ResourceStatus::Running;
                let mut end = false;
                let mut reason = LimitReason::Mem;

                if mem.oom {
                    log::trace!("Stopping process because it reach its memory limit");
                    reason = LimitReason::Mem;
                    end = true;
                } else if cpu.rt_us > limit.rt_us
                    || cpu.cpu_us > limit.cpu_us
                    || cpu.total_us > limit.total_us
                {
                    log::trace!("Killing process because it reach its cpu quota");
                    dbg!(&cpu);
                    dbg!(&limit);
                    reason = LimitReason::Cpu;
                    end = true;
                }

                unsafe {
                    let state_ptr = Box::into_raw(Box::new(LimiterState { cpu, mem }));
                    drop(Box::from_raw(
                        state_taken.swap(state_ptr, Ordering::Relaxed),
                    ));
                }
                if end {
                    tx.send(reason).ok();
                    cg.kill().unwrap();
                    log::trace!("Process was killed");
                    break;
                }
            }
        });

        Ok(Limiter {
            task,
            state,
            limit_oneshot: Some(rx),
        })
    }
    pub async fn status(self) -> (CpuStatistics, MemStatistics) {
        let stat = unsafe { Box::from_raw(self.state.load(Ordering::SeqCst)) };

        (stat.cpu.clone(), stat.mem.clone())
    }
    pub fn wait_exhausted(&mut self) -> Receiver<LimitReason> {
        self.limit_oneshot
            .take()
            .expect("Limiter cannot be wait twice!")
    }
}
