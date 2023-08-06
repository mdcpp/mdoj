use cgroups_rs::{memory::MemController, Cgroup};

#[derive(Default, Clone)]
pub struct MemStatistics {
    pub oom: bool,
    pub peak: u64,
}

impl MemStatistics {
    pub fn from_cgroup(cgroup: &Cgroup) -> Self {
        let ctrl = cgroup.controller_of().unwrap();

        Self::from_controller(ctrl)
    }
    pub fn from_controller(mem: &MemController) -> Self {
        let stat = mem.memory_stat();

        let oom = stat.oom_control.oom_kill != 0;
        let peak = stat.max_usage_in_bytes;

        Self { oom, peak }
    }
}

// pub struct MemLimiter {}

// impl MemLimiter {
//     fn check(cg:&Cgroup){
//         let m:&MemController=cg.controller_of().unwrap();
//         m.kmem_stat()
//     }
// }
