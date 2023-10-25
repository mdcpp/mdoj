use std::path::Path;

use cgroups_rs::{hierarchies, Hierarchy};

use super::config::CONFIG;

pub fn init() {
    let config = CONFIG.get().unwrap();

    if config.nsjail.is_cgv1() {
        let hier = hierarchies::V1::new();
        let subsystems = hier.subsystems();
        if let None = subsystems.iter().find(|sub| match sub {
            cgroups_rs::Subsystem::CpuAcct(_) => true,
            _ => false,
        }) {
            log::warn!("Subsystem CpuAcct(Cpu Accounting) is unavailable.");
        };

        if let None = subsystems.iter().find(|sub| match sub {
            cgroups_rs::Subsystem::CpuSet(_) => true,
            _ => false,
        }) {
            log::warn!("Subsystem CpuSet(Cpu Scheduling Per Core) is unavailable.");
        };

        if let None = subsystems.iter().find(|sub| match sub {
            cgroups_rs::Subsystem::Cpu(_) => true,
            _ => false,
        }) {
            log::warn!("Subsystem Cpu(Cpu Scheduling) is unavailable.");
        };

        if let None = subsystems.iter().find(|sub| match sub {
            cgroups_rs::Subsystem::Mem(_) => true,
            _ => false,
        }) {
            log::warn!("Subsystem Mem(Memory) is unavailable.");
        };
    } else {
        let hier = hierarchies::V2::new();
        let subsystems = hier.subsystems();
        if let None = subsystems.iter().find(|sub| match sub {
            cgroups_rs::Subsystem::CpuSet(_) => true,
            _ => false,
        }) {
            log::warn!("Subsystem CpuSet(Cpu Scheduling Per Core) is unavailable.");
        };

        if let None = subsystems.iter().find(|sub| match sub {
            cgroups_rs::Subsystem::Cpu(_) => true,
            _ => false,
        }) {
            log::warn!("Subsystem Cpu(Cpu Scheduling) is unavailable.");
        };

        if let None = subsystems.iter().find(|sub| match sub {
            cgroups_rs::Subsystem::Mem(_) => true,
            _ => false,
        }) {
            log::warn!("Subsystem Mem(Memory) is unavailable.");
        };
    }

    if !config.nsjail.rootless {
        let uid = unsafe { libc::getuid() };
        if uid != 0 {
            log::warn!("config.rootless is set to false, require root to run");
        }
    } else {
        let root_cg = Path::new("/sys/fs/cgroup").join(config.runtime.root_cgroup.clone());

        match root_cg.metadata() {
            Ok(meta) => {
                if meta.permissions().readonly() {
                    log::warn!("config.rootless is set to true, but cgroup root is readonly, please either set config.rootless to false or make cgroup root writable");
                }
            }
            Err(x) => log::error!("Unable to find cgroup root, is it mounted? {}", x),
        }
    }
}
