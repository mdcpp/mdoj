use std::{
    ops::{AddAssign, Div, Mul},
    time::Duration,
};

use cgroups_rs::cpuacct::CpuAcct;

use super::output::Output;

pub type MemAndCpu = (Memory, Cpu);

/// collections of resource usage
///
/// basically, it contains memory usage, cpu usage, output size and walltime
#[derive(Clone, Default, Debug)]
pub struct Stat {
    pub memory: Memory,
    pub cpu: Cpu,
    pub output: Output,
    pub walltime: Duration,
}

impl AddAssign<Stat> for Stat {
    fn add_assign(&mut self, rhs: Stat) {
        self.memory += rhs.memory;
        self.cpu += rhs.cpu;
        self.output += rhs.output;
        self.walltime += rhs.walltime;
    }
}

/// memory usage(in bytes)
#[derive(Clone, Default, Debug)]
pub struct Memory {
    pub kernel: u64,
    pub user: u64,
    pub total: u64,
}

impl AddAssign<Memory> for Memory {
    fn add_assign(&mut self, rhs: Memory) {
        self.kernel += rhs.kernel;
        self.user += rhs.user;
        self.total += rhs.total;
    }
}

impl Mul<f64> for Memory {
    type Output = Memory;

    fn mul(self, rhs: f64) -> Self::Output {
        Memory {
            kernel: (self.kernel as f64 * rhs) as u64,
            user: (self.user as f64 * rhs) as u64,
            total: (self.total as f64 * rhs) as u64,
        }
    }
}

impl Div<f64> for Memory {
    type Output = Memory;

    fn div(self, rhs: f64) -> Self::Output {
        Memory {
            kernel: (self.kernel as f64 / rhs) as u64,
            user: (self.user as f64 / rhs) as u64,
            total: (self.total as f64 / rhs) as u64,
        }
    }
}

impl Memory {
    pub fn get_reserved_size(&self) -> u64 {
        self.total.min(self.user + self.kernel)
    }
}

/// cpu usage(in nanoseconds)
#[derive(Clone, Default, Debug)]
pub struct Cpu {
    pub kernel: u64,
    pub user: u64,
    pub total: u64,
}

impl AddAssign<Cpu> for Cpu {
    fn add_assign(&mut self, rhs: Cpu) {
        self.kernel += rhs.kernel;
        self.user += rhs.user;
        self.total += rhs.total;
    }
}

impl Mul<f64> for Cpu {
    type Output = Cpu;

    fn mul(self, rhs: f64) -> Self::Output {
        Cpu {
            kernel: (self.kernel as f64 * rhs) as u64,
            user: (self.user as f64 * rhs) as u64,
            total: (self.total as f64 * rhs) as u64,
        }
    }
}

impl Div<f64> for Cpu {
    type Output = Cpu;

    fn div(self, rhs: f64) -> Self::Output {
        Cpu {
            kernel: (self.kernel as f64 / rhs) as u64,
            user: (self.user as f64 / rhs) as u64,
            total: (self.total as f64 / rhs) as u64,
        }
    }
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
