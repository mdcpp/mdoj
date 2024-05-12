use std::collections::BTreeMap;

use tokio::{fs::File, sync::Semaphore};
use uuid::Uuid;

use super::spec::*;
use crate::{filesystem::Template, CONFIG};

static PLUGIN_PATH: &str = "./plugins";
/// max queue judging task
const MAX_QUEUE: usize = 10;

pub struct Daemon {
    semaphore: Semaphore,
    templates: BTreeMap<Uuid, Spec>,
}

struct Plugin {
    config: Spec,
    template: Template<File>,
}

impl Daemon {
    pub fn new() -> Self {
        let semaphore = Semaphore::new(todo!());
        let mut templates = BTreeMap::new();
        todo!("Load plugins");
        // design a loader struct
        Daemon {
            semaphore,
            templates,
        }
    }
}
