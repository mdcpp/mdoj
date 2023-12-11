use std::path::Path;

use cgroups_rs::{hierarchies, Hierarchy, Subsystem};

use super::config::CONFIG;

// Check if all required systems are met
// abort if necessary
pub fn init() {
    let config = CONFIG.get().unwrap();

    if config.nsjail.is_cgv1() {
        let hier = hierarchies::V1::new();
        let subsystems = hier.subsystems();
        if subsystems
            .iter()
            .any(|sub| matches!(sub, Subsystem::CpuAcct(_)))
        {
            log::warn!("Subsystem CpuAcct(Cpu Accounting) is unavailable.");
        };

        if subsystems
            .iter()
            .any(|sub| matches!(sub, Subsystem::CpuSet(_)))
        {
            log::warn!("Subsystem CpuSet(Cpu Scheduling Per Core) is unavailable.");
        };

        if subsystems
            .iter()
            .any(|sub| matches!(sub, Subsystem::Cpu(_)))
        {
            log::warn!("Subsystem Cpu(Cpu Scheduling) is unavailable.");
        };

        if subsystems
            .iter()
            .any(|sub| matches!(sub, Subsystem::Mem(_)))
        {
            log::warn!("Subsystem Mem(Memory) is unavailable.");
        };
        log::error!("cgroup v1 is not supported, it fail at cpu task");
        std::process::exit(1);
    } else {
        let hier = hierarchies::V2::new();
        let subsystems = hier.subsystems();
        if subsystems
            .iter()
            .any(|sub| matches!(sub, Subsystem::CpuSet(_)))
        {
            log::warn!("Subsystem CpuSet(Cpu Scheduling Per Core) is unavailable.");
        };

        if subsystems
            .iter()
            .any(|sub| matches!(sub, Subsystem::Cpu(_)))
        {
            log::warn!("Subsystem Cpu(Cpu Scheduling) is unavailable.");
        };

        if subsystems
            .iter()
            .any(|sub| matches!(sub, Subsystem::Mem(_)))
        {
            log::warn!("Subsystem Mem(Memory) is unavailable.");
        };
    }

    if !config.nsjail.rootless {
        let uid = rustix::process::getuid();
        if !uid.is_root() {
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

    if config.platform.output_limit >= config.platform.available_memory.try_into().unwrap() {
        log::error!("config.platform.output_limit is too larget or config.platform.available_memory is too low");
        std::process::exit(1);
        }

    if config.platform.output_limit * 8 >= config.platform.available_memory.try_into().unwrap() {
        log::warn!("config.platform.output_limit is consider too high");
    }
}
