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
    pub fn oom(&self) -> std::sync::mpsc::Receiver<String> {
        let controller = self.cgroup.controller_of::<MemController>().unwrap();
        controller.register_oom_event("mdoj-oom-handler").unwrap()
    }
    pub fn memory(&self) -> Memory {
        let controller = self.cgroup.controller_of::<MemController>().unwrap();
        let kusage = controller.kmem_stat();

        let kernel = kusage.max_usage_in_bytes as u64;
        let user = controller.memory_stat().max_usage_in_bytes;
        let total = kernel + user;

        Memory {
            kernel,
            user,
            total,
        }
    }
}
