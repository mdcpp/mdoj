use super::hier::*;
use super::stat::*;
use cgroups_rs::{cpu::CpuController, cpuacct::CpuAcctController, memory::MemController, Cgroup};
use std::ops::Deref;

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
            MonitorKind::Cpu => {
                let controller: &CpuAcctController = self.cgroup.controller_of().unwrap();
                Cpu::from_acct(controller.cpuacct())
            }
            MonitorKind::CpuAcct => {
                let controller: &CpuController = self.cgroup.controller_of().unwrap();
                let raw: &str = &controller.cpu().stat;
                Cpu::from_raw(raw)
            }
        }
    }
    /// get an receiver(synchronize) for oom event
    pub fn oom_signal(&self) -> std::sync::mpsc::Receiver<String> {
        let controller = self.cgroup.controller_of::<MemController>().unwrap();
        controller.register_oom_event("mdoj-oom-handler").unwrap()
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