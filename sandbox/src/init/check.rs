use cgroups_rs::{hierarchies, Hierarchy};

pub fn init() {
    let hier = hierarchies::V2::new();
    let subsystems = hier.subsystems();
    if let None = subsystems.iter().find(|sub| match sub {
        cgroups_rs::Subsystem::CpuAcct(_) => true,
        _ => false,
    }) {
        log::warn!("Subsystem CpuAcct(Cpu Accounting) is unavailable, if the program is using CGroupv2, it's safe to ignore this warning.");
    };

    if let None = subsystems.iter().find(|sub| match sub {
        cgroups_rs::Subsystem::Cpu(_) => true,
        _ => false,
    }) {
        log::error!("Subsystem Cpu(Cpu Scheduling) is unavailable.");
    };

    if let None = subsystems.iter().find(|sub| match sub {
        cgroups_rs::Subsystem::Mem(_) => true,
        _ => false,
    }) {
        log::error!("Subsystem Mem(Memory) is unavailable.");
    };
}
