use cgroups_rs::{cpu::CpuController, cpuacct::CpuAcctController, memory::MemController, Cgroup};

use super::stat::*;

/// newtype wrapper for cgroup form cgroup_rs
pub struct CgroupWrapper<'a> {
    cgroup: &'a Cgroup,
}

impl<'a> CgroupWrapper<'a> {
    pub fn new(cgroup: &'a Cgroup) -> Self {
        Self { cgroup }
    }
    pub fn cpu(&self) -> Cpu {
        let mut kernel = u64::MAX;
        let mut user = u64::MAX;
        let mut total = u64::MAX;

        match self.cgroup.controller_of::<CpuAcctController>() {
            Some(controller) => {
                let usage = controller.cpuacct();
                kernel = usage.usage_sys;
                user = usage.usage_user;
                total = usage.usage;
            }
            None => {
                let controller: &CpuController = self.cgroup.controller_of().unwrap();

                let raw: &str = &controller.cpu().stat;

                for (key, value) in raw.split('\n').filter_map(|stmt| stmt.split_once(' ')) {
                    match key {
                        "usage_usec" => total = value.parse().unwrap(),
                        "user_usec" => user = value.parse().unwrap(),
                        "system_usec" => kernel = value.parse().unwrap(),
                        _ => {}
                    };
                }
            }
        }

        Cpu {
            kernel,
            user,
            total,
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
