use crate::config::Accounting;
use cgroups_rs::*;

/// type of monitor for cpu
pub enum MonitorKind {
    /// use `cpu.stat` from cpu subsystem
    Cpu,
    /// use cpu accounting subsystem
    CpuAcct,
}

lazy_static::lazy_static! {
    pub static ref MONITER_KIND: MonitorKind =
        match crate::CONFIG.accounting {
            Accounting::Auto =>match hierarchies::auto().v2(){
                true=>MonitorKind::Cpu,
                false=>MonitorKind::CpuAcct
            },
            Accounting::CpuAccounting => MonitorKind::CpuAcct,
            Accounting::Cpu => MonitorKind::Cpu,
        };
}

impl MonitorKind {
    /// get the hierarchy(cgroup v1/v2) of monitor
    pub fn heir(&self) -> Box<dyn Hierarchy> {
        match self {
            MonitorKind::Cpu => hierarchies::auto(),
            MonitorKind::CpuAcct => Box::new(hierarchies::V1::new()),
        }
    }
}
