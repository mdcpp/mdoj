use std::fmt::Display;

use cgroups_rs::{cpu::CpuController, cpuacct::CpuAcctController, Cgroup};

use crate::init::config::CONFIG;

#[derive(Default, Clone, Debug)]
pub struct CpuStatistics {
    pub rt_us: i64,
    pub cpu_us: u64,
    pub total_us: u64,
}

impl Display for CpuStatistics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "realtime:{} ,user: {} , total: {}",
            self.rt_us, self.cpu_us, self.total_us
        )
    }
}

impl CpuStatistics {
    pub fn from_cgroup(cgroup: &Cgroup) -> Self {
        let config = CONFIG.get().unwrap();
        if config.nsjail.is_cgv1() {
            let ctrl = cgroup.controller_of().unwrap();
            Self::from_cpuacct_controller(ctrl)
        } else {
            let ctrl = cgroup.controller_of().unwrap();
            Self::from_cpu_controller(ctrl)
        }
    }
    pub fn from_cpuacct_controller(cpuacct: &CpuAcctController) -> Self {
        let acct = cpuacct.cpuacct();

        Self {
            rt_us: acct.usage_sys as i64,
            cpu_us: acct.usage_user,
            total_us: acct.usage,
        }
    }
    pub fn from_cpu_controller(cpu: &CpuController) -> Self {
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
