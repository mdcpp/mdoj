use std::{
    fmt::Display,
    ops::{Div, Mul},
    sync::{Arc, Mutex},
};

use crate::init::config::CONFIG;

use super::Error;
use cgroups_rs::{
    cgroup_builder::CgroupBuilder, cpu::CpuController, hierarchies, memory::MemController, Cgroup,
    CgroupPid,
};
use tokio::{process::Child, sync::oneshot, task::JoinHandle, time};

use super::cpuacct::{CpuAcct, CpuStatKey};

const INTERVAL: u64 = 50 * 1000; // 50ms

#[derive(Clone, Debug, Default)]
pub struct CpuLimit {
    pub cpu_us: u64,
    pub rt_us: i64,
    pub total_us: u64,
}

impl Mul<u64> for CpuLimit {
    type Output = CpuLimit;

    fn mul(self, rhs: u64) -> Self::Output {
        Self {
            cpu_us: self.cpu_us * rhs,
            rt_us: self.rt_us * (rhs as i64),
            total_us: self.total_us * rhs,
        }
    }
}

impl Div<u64> for CpuLimit {
    type Output = CpuLimit;

    fn div(self, rhs: u64) -> Self::Output {
        Self {
            cpu_us: self.cpu_us / rhs,
            rt_us: self.rt_us / (rhs as i64),
            total_us: self.total_us / rhs,
        }
    }
}

impl CpuLimit {
    fn to_cgroup(&self, cgroup_name: &str, memory: MemLimit) -> Result<Cgroup, Error> {
        let config = CONFIG.get().unwrap();

        log::trace!("Creating new control group {}", cgroup_name);
        let hier = Box::new(hierarchies::V2::new());

        Ok(CgroupBuilder::new(&cgroup_name)
            .memory()
            .kernel_memory_limit(memory.kernel)
            .memory_hard_limit(memory.user)
            .memory_swap_limit(memory.swap)
            .done()
            .cpu()
            .period(config.runtime.accuracy)
            .quota(config.runtime.accuracy as i64)
            .realtime_period(config.runtime.accuracy)
            .realtime_runtime(config.runtime.accuracy as i64)
            .done()
            .build(hier)?)
    }
    pub fn max() -> Self {
        Self {
            cpu_us: u64::MIN / 2 - 1,
            rt_us: i64::MIN / 2 - 1,
            total_us: u64::MIN / 2 - 1,
        }
    }
}

pub struct MemLimit {
    pub user: i64,
    pub kernel: i64,
    pub swap: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LimitReason {
    Memory,
    CpuTime(u64),
    RealTime(i64),
    TotalTime(u64),
}

impl Display for LimitReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LimitReason::Memory => write!(f, "Memory limit reached"),
            LimitReason::CpuTime(x) => {
                write!(f, "Cpu time excess ({} microseconds has been consumed)", x)
            }
            LimitReason::RealTime(x) => write!(
                f,
                "Realtime(kernerl) cpu time excess ({} microseconds has been consumed)",
                x
            ),
            LimitReason::TotalTime(x) => write!(
                f,
                "Total cpu time excess ({} microseconds has been consumed)",
                x
            ),
        }
    }
}

#[derive(Default)]
struct Status {
    oneshot: Vec<oneshot::Sender<()>>,
    reason: Option<LimitReason>,
    cpu_usage: CpuLimit,
}

impl Status {
    fn exhaust(&mut self, reason: LimitReason) {
        self.reason = Some(reason);
    }
    async fn wait_exhaust(self_: &Mutex<Self>) {
        let (tx, rx) = oneshot::channel();
        {
            self_.lock().unwrap().oneshot.push(tx)
        };
        rx.await.unwrap()
    }
}

pub struct Limiter {
    cgroup: Cgroup,
    handle: Option<JoinHandle<()>>,
    status: Arc<Mutex<Status>>,
    jails_pids: Vec<i32>
}

impl Drop for Limiter {
    fn drop(&mut self) {
        self.cgroup.kill().unwrap();
        self.cgroup.delete().unwrap();
        self.handle.take().unwrap().abort();
    }
}

impl Limiter {
    pub fn cpu_usage(&self) -> CpuLimit {
        self.status.as_ref().lock().unwrap().cpu_usage.clone()
    }
    pub fn from_limit(cgroup_name: &str, cpu: CpuLimit, memory: MemLimit) -> Result<Self, Error> {
        let config = CONFIG.get().unwrap();

        log::debug!("Starting a new limiter");

        let cgroup = cpu.to_cgroup(cgroup_name, memory)?;
        let status = Arc::new(Mutex::new(Status::default()));
        let handle_status = status.clone();

        let mut state: State = State {
            cgroup: cgroup.clone(),
            record_round: 0,
            status: status.clone(),
        };

        let handle = tokio::spawn(async move {
            log::trace!("Cpu resource monitor started");
            let limit = cpu.clone();
            loop {
                if let Some(reason) = state.check(&limit).unwrap() {
                    handle_status.lock().unwrap().exhaust(reason);
                    break;
                }
                time::sleep(time::Duration::from_nanos(config.runtime.accuracy / 2)).await;
            }
            log::trace!("Killing process inside cgroup");
            state.cgroup.kill().unwrap();
            log::trace!("Killing nsjail");
            todo!();
        });

        Ok(Limiter {
            cgroup,
            handle: Some(handle),
            status,
            jails_pids:Vec::new()
        })
    }
    pub fn add_child(&self, child: &Child) -> Result<(), Error> {
        self.cgroup
            .add_task_by_tgid(CgroupPid {
                pid: child.id().unwrap() as u64,
            })
            .map_err(|_| Error::CGroup)
    }
    pub fn status(&self) -> Option<LimitReason> {
        self.status.lock().unwrap().reason.clone()
    }
    pub async fn wait(&self) {
        Status::wait_exhaust(self.status.as_ref()).await
    }
}

#[derive(Default)]
struct State {
    cgroup: Cgroup,
    record_round: i64,
    status: Arc<Mutex<Status>>,
}

impl State {
    fn check_cpu(&mut self, cap: &CpuLimit) -> Result<Option<LimitReason>, Error> {
        let config = CONFIG.get().unwrap();

        let cpu: &CpuController = self.cgroup.controller_of().ok_or(Error::CGroup)?;
        let cpuacct = CpuAcct::from_controller(cpu);

        let previous_limit = &mut self.status.as_ref().lock().unwrap().cpu_usage;

        self.record_round = cpuacct.get(CpuStatKey::NrPeriods).ok_or(Error::CGroup)?;

        *previous_limit = CpuLimit {
            cpu_us: cpuacct.get(CpuStatKey::UserUsec).ok_or(Error::CGroup)? as u64,
            rt_us: cpuacct.get(CpuStatKey::SystemUsec).ok_or(Error::CGroup)?,
            total_us: cpuacct.get(CpuStatKey::UsageUsec).ok_or(Error::CGroup)? as u64,
        } / config.platform.cpu_time_multiplier;

        Ok(if cap.cpu_us < previous_limit.cpu_us {
            log::trace!("Cpu Resource limit reach");
            Some(LimitReason::CpuTime(previous_limit.cpu_us))
        } else if cap.rt_us < previous_limit.rt_us {
            log::trace!("Cpu Resource limit reach");
            Some(LimitReason::RealTime(previous_limit.rt_us))
        } else if cap.total_us < previous_limit.total_us {
            log::trace!("Cpu Resource limit reach");
            Some(LimitReason::TotalTime(previous_limit.total_us))
        } else {
            None
        })
    }
    fn check_mem(&mut self) -> Result<Option<LimitReason>, Error> {
        let mem: &MemController = self.cgroup.controller_of().ok_or(Error::CGroup)?;
        let stat = mem.memory_stat();

        Ok(if stat.fail_cnt > 0 {
            log::trace!("Memory Resource limit reach");
            Some(LimitReason::Memory)
        } else {
            None
        })
    }
    fn check(&mut self, cpu: &CpuLimit) -> Result<Option<LimitReason>, Error> {
        Ok(if let Some(x) = self.check_cpu(cpu)? {
            Some(x)
        } else if let Some(x) = self.check_mem()? {
            Some(x)
        } else {
            None
        })
    }
}

#[derive(Default)]
struct LimitCounter {
    round: usize,
}
