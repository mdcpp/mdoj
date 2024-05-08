use std::collections::BTreeMap;

use uuid::Uuid;

use super::config::*;
use crate::semaphore::Semaphore;
use crate::CONFIG;

static PLUGIN_PATH: &str = "./plugins";
/// max queue judging task
const MAX_QUEUE: usize = 10;

pub struct Daemon {
    semaphore: Semaphore,
    templates: BTreeMap<Uuid, Config>,
}

impl Daemon {
    pub fn new() -> Self {
        let semaphore = Semaphore::new(CONFIG.memory, MAX_QUEUE);
        let mut templates = BTreeMap::new();
        todo!("Load plugins");
        // design a loader struct
        Daemon {
            semaphore,
            templates,
        }
    }
}
