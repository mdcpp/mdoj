use crate::async_loop;

use super::{hier::*, stat::*};
use cgroups_rs::{cpu::CpuController, cpuacct::CpuAcctController, memory::MemController, Cgroup};
use std::{ops::Deref, pin::pin};
use tokio::task::JoinHandle;

pub struct OOMSignal {
    rx: Option<JoinHandle<()>>,
}

impl Drop for OOMSignal {
    fn drop(&mut self) {
        self.rx.take().unwrap().abort();
    }
}

impl OOMSignal {
    fn new(rx: JoinHandle<()>) -> Self {
        Self { rx: Some(rx) }
    }
    pub async fn wait(mut self) {
        let rx = pin!(self.rx.as_mut().unwrap());
        rx.await.ok();
    }
}

/// newtype wrapper for cgroup form cgroup_rs
pub struct CgroupWrapper<'a> {
    cgroup: &'a Cgroup,
}

impl<'a> CgroupWrapper<'a> {
    pub fn new(cgroup: &'a Cgroup) -> Self {
        Self { cgroup }
    }
    /// get cpu usage(statistics)
    pub fn cpu(&self) -> Cpu {
        match MONITER_KIND.deref() {
            MonitorKind::CpuAcct => {
                let controller: &CpuAcctController = self.cgroup.controller_of().unwrap();
                Cpu::from_acct(controller.cpuacct())
            }
            MonitorKind::Cpu => {
                let controller: &CpuController = self.cgroup.controller_of().unwrap();
                let raw: &str = &controller.cpu().stat;
                Cpu::from_raw(raw)
            }
        }
    }
    /// get an receiver(synchronize) for oom event
    pub fn oom_signal(&self) -> OOMSignal {
        let controller = self.cgroup.controller_of::<MemController>().unwrap();
        if self.cgroup.v2() {
            let controller = controller.to_owned();
            OOMSignal::new(tokio::spawn(async_loop!({
                if controller.memory_stat().oom_control.oom_kill != 0 {
                    break;
                }
            })))
        } else {
            let oom_signal = controller.register_oom_event("mdoj_oom").unwrap();
            OOMSignal::new(tokio::task::spawn_blocking(move || {
                oom_signal.recv().ok();
            }))
        }
    }
    /// get memory usage(statistics)
    pub fn memory(&self) -> Memory {
        let controller = self.cgroup.controller_of::<MemController>().unwrap();
        let kusage = controller.kmem_stat();

        let kernel = kusage.max_usage_in_bytes;
        let user = controller.memory_stat().max_usage_in_bytes;
        let total = kernel + user;

        Memory {
            kernel,
            user,
            total,
        }
    }
    /// check if oom
    ///
    /// use [`oom_signal`] if long polling is required
    pub fn oom(&self) -> bool {
        let controller: &MemController = self.cgroup.controller_of().unwrap();
        controller.memory_stat().oom_control.oom_kill != 0
    }
}
