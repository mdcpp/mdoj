use std::time::Duration;

use cgroups_rs::cpuacct::CpuAcct;

use super::output::Output;

pub type MemAndCpu = (Memory, Cpu);

pub struct Stat {
    pub memory: Memory,
    pub cpu: Cpu,
    pub output: Output,
    pub walltime: Duration,
}

pub struct Memory {
    pub kernel: u64,
    pub user: u64,
    pub total: u64,
}

impl Memory {
    pub fn get_reserved_size(&self) -> u64 {
        self.total.min(self.user + self.kernel)
    }
}

#[derive(Clone)]
pub struct Cpu {
    pub kernel: u64,
    pub user: u64,
    pub total: u64,
}

impl Cpu {
    pub(super) fn out_of_resources(resource: &Self, stat: Self) -> bool {
        stat.kernel > resource.kernel || stat.user > resource.user || stat.total > resource.total
    }

    pub(super) fn from_acct(acct: CpuAcct) -> Self {
        Cpu {
            kernel: acct.usage_sys,
            user: acct.usage_user,
            total: acct.usage,
        }
    }
    pub(super) fn from_raw(raw: &str) -> Self {
        let mut kernel = u64::MAX;
        let mut user = u64::MAX;
        let mut total = u64::MAX;

        for (key, value) in raw.split('\n').filter_map(|stmt| stmt.split_once(' ')) {
            match key {
                "usage_usec" => total = value.parse().unwrap(),
                "user_usec" => user = value.parse().unwrap(),
                "system_usec" => kernel = value.parse().unwrap(),
                _ => {}
            };
        }

        Self {
            kernel,
            user,
            total,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    /// Test the [`Cpu::from_raw`] function
    fn cpu_from_raw() {
        let raw = "usage_usec 158972260000\nuser_usec 115998852000\nsystem_usec 42973408000\ncore_sched.force_idle_usec 0\nnr_periods 0\nnr_throttled 0\nthrottled_usec 0\nnr_bursts 0\nburst_usec 0\n";
        let cpu = Cpu::from_raw(raw);
        assert_eq!(cpu.kernel, 42973408000);
        assert_eq!(cpu.user, 115998852000);
        assert_eq!(cpu.total, 158972260000);
    }
}
