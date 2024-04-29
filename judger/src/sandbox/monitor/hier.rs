use crate::config::Accounting;
use cgroups_rs::*;

pub enum MonitorKind {
    Cpu,
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
    pub fn heir(&self) -> Box<dyn Hierarchy> {
        match self {
            MonitorKind::Cpu => hierarchies::auto(),
            MonitorKind::CpuAcct => Box::new(hierarchies::V1::new()),
        }
    }
}
