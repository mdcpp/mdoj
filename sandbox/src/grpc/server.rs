use std::collections::BTreeMap;

use crate::{init::config::CONFIG, jail::prelude::*, langs::spec::RawLangSpec};

pub type UUID = String;
pub struct GRpcServer {
    langs: BTreeMap<UUID, RawLangSpec>,
    runtime: ContainerDaemon,
}

impl Default for GRpcServer {
    fn default() -> Self {
        let config = CONFIG.get().unwrap();
        let runtime = ContainerDaemon::new(&config.runtime.temp);
        Self {
            langs: Default::default(),
            runtime,
        }
    }
}

impl GRpcServer {}
