use std::num::NonZeroUsize;

use lru::LruCache;
use spin::mutex::Mutex;
use uuid::Uuid;

pub struct DupController {
    #[cfg(feature = "single-instance")]
    dups: Mutex<LruCache<(i32, Uuid), i32>>,
}

impl DupController {
    pub fn new() -> Self {
        log::debug!("Setup DupController");
        Self {
            #[cfg(feature = "single-instance")]
            dups: Mutex::new(LruCache::new(NonZeroUsize::new(100).unwrap())),
        }
    }
    pub fn store(&self, user_id: i32, uuid: Uuid, result: i32) {
        #[cfg(feature = "single-instance")]
        self.dups.lock().put((user_id, uuid), result);
    }
    pub fn check(&self, user_id: i32, uuid: &Uuid) -> Option<i32> {
        #[cfg(feature = "single-instance")]
        if let Some(x) = self.dups.lock().get(&(user_id, *uuid)) {
            return Some(*x);
        }
        None
    }
}
