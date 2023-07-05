use crate::limit::Error;
use crate::limit::Limit;
use std::sync::atomic::AtomicPtr;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use cgroups_rs::{cgroup_builder::CgroupBuilder, hierarchies};
use tokio::task::JoinHandle;
use tokio::time;

use crate::limit::utils::limiter::cpu::CpuStatistics;
use crate::limit::utils::limiter::mem::MemStatistics;
use crate::{init::config::CONFIG, limit::proc::ProcState};

pub mod cpu;
pub mod mem;

pub struct Limiter {
    task: JoinHandle<()>,
    proc_state: Arc<ProcState>,
    state: Arc<AtomicPtr<LimiterState>>,
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
    pub fn new(cg_name: &str, limit: Limit, proc_state: Arc<ProcState>) -> Result<Self, Error> {
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

        let proc_state_taken = proc_state.clone();
        let state_taken = state.clone();
        let task = tokio::spawn(async move {
            loop {
                time::sleep(time::Duration::from_nanos(config.runtime.accuracy)).await;

                let cpu = CpuStatistics::from_cgroup(&cg);
                let mem = MemStatistics::from_cgroup(&cg);

                // let mut resource_status = ResourceStatus::Running;
                let mut end = false;

                if mem.oom {
                    // resource_status = ResourceStatus::MemExhausted;
                    end = true;
                } else if cpu.rt_us > limit.rt_us
                    || cpu.cpu_us > limit.cpu_us
                    || cpu.total_us > limit.total_us
                {
                    // resource_status = ResourceStatus::CpuExhausted;
                    end = true;
                }

                unsafe {
                    let state_ptr = Box::into_raw(Box::new(LimiterState { cpu, mem }));
                    drop(Box::from_raw(
                        state_taken.swap(state_ptr, Ordering::Relaxed),
                    ));
                }
                if end {
                    proc_state_taken.nsjail.kill().await.ok();
                    break;
                }
            }
        });

        Ok(Limiter {
            task,
            proc_state,
            state,
        })
    }
    pub async fn status(self) -> (CpuStatistics, MemStatistics) {
        let stat = unsafe { Box::from_raw(self.state.load(Ordering::SeqCst)) };

        (stat.cpu.clone(), stat.mem.clone())
    }
}
