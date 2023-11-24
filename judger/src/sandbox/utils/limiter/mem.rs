use cgroups_rs::{memory::MemController, Cgroup};

#[derive(Default, Clone, Debug)]
pub struct MemStatistics {
    pub oom: bool,
    pub peak: u64,
}

impl MemStatistics {
    // generate memory statistics from cgroup
    pub fn from_cgroup(cgroup: &Cgroup) -> Self {
        let ctrl = cgroup.controller_of().unwrap();

        Self::from_controller(ctrl)
    }
    // generate memory statistics with memory controller
    pub fn from_controller(mem: &MemController) -> Self {
        let stat = mem.memory_stat();

        let oom = stat.oom_control.oom_kill != 0;
        let peak = stat.max_usage_in_bytes;

        Self { oom, peak }
    }
}
