use cgroups_rs::{cpu::CpuController, Cgroup};

#[derive(Default, Clone,Debug)]
pub struct CpuStatistics {
    pub rt_us: i64,
    pub cpu_us: u64,
    pub total_us: u64,
}

impl CpuStatistics {
    pub fn from_cgroup(cgroup: &Cgroup) -> Self {
        let ctrl = cgroup.controller_of().unwrap();

        Self::from_controller(ctrl)
    }
    pub fn from_controller(cpu: &CpuController) -> Self {
        let raw: &str = &cpu.cpu().stat;
        let mut rt_us = i64::MAX;
        let mut cpu_us = u64::MAX;
        let mut total_us = u64::MAX;
        for (key, value) in raw
            .split("\n")
            .filter_map(|stmt| match stmt.split_once(" ") {
                Some(a) => Some(a),
                None => None,
            })
        {
            match key {
                "usage_usec" => total_us = value.parse().unwrap(),
                "user_usec" => cpu_us = value.parse().unwrap(),
                "system_usec" => rt_us = value.parse().unwrap(),
                _ => {}
            };
        }
        Self {
            rt_us,
            cpu_us,
            total_us,
        }
    }
}
