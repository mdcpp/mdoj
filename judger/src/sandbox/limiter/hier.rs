use cgroups_rs::*;
use std::time::Duration;

pub const MONITOR_ACCURACY: Duration = Duration::from_millis(80);

pub enum MonitorKind {
    Cpu,
    CpuAcct,
}

lazy_static::lazy_static! {
    pub static ref MONITER_KIND: MonitorKind =
        match hierarchies::auto().v2(){
            true=>MonitorKind::Cpu,
            false=>MonitorKind::CpuAcct
        }
    ;
}

impl MonitorKind {
    pub fn heir(&self) -> Box<dyn Hierarchy> {
        match self {
            MonitorKind::Cpu => hierarchies::auto(),
            MonitorKind::CpuAcct => Box::new(hierarchies::V1::new()),
        }
    }
}
