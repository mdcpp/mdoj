use quick_cache::sync::Cache;
use uuid::Uuid;

pub struct DupController {
    #[cfg(feature = "single-instance")]
    dups: Cache<(i32, Uuid), i32>,
}

impl Default for DupController {
    fn default() -> Self {
        log::debug!("Setup DupController");
        Self {
            #[cfg(feature = "single-instance")]
            dups: Cache::new(300),
        }
    }
}

impl DupController {
    pub fn store(&self, user_id: i32, uuid: Uuid, result: i32) {
        #[cfg(feature = "single-instance")]
        self.dups.insert((user_id, uuid), result);
    }
    pub fn check(&self, user_id: i32, uuid: &Uuid) -> Option<i32> {
        #[cfg(feature = "single-instance")]
        if let Some(x) = self.dups.get(&(user_id, *uuid)) {
            return Some(x);
        }
        None
    }
}
