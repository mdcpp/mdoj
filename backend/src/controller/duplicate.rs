use quick_cache::sync::Cache;
use tracing::Span;
use uuid::Uuid;

pub struct DupController {
    #[cfg(feature = "single-instance")]
    dup_i32: Cache<(i32, Uuid), i32>,
    #[cfg(feature = "single-instance")]
    dup_str: Cache<(i32, Uuid), String>,
}

impl DupController {
    #[tracing::instrument(parent=span, name="duplicate_construct",level = "info",skip_all)]
    pub fn new(span: &Span) -> Self {
        Self {
            #[cfg(feature = "single-instance")]
            dup_i32: Cache::new(150),
            #[cfg(feature = "single-instance")]
            dup_str: Cache::new(150),
        }
    }
    /// store a request_id with result i32
    pub fn store_i32(&self, spliter: i32, uuid: Uuid, result: i32) {
        tracing::trace!(request_id=?uuid);
        #[cfg(feature = "single-instance")]
        self.dup_i32.insert((spliter, uuid), result);
    }
    /// store a request_id with result String
    pub fn store_str(&self, spliter: i32, uuid: Uuid, result: String) {
        tracing::trace!(request_id=?uuid);
        #[cfg(feature = "single-instance")]
        self.dup_str.insert((spliter, uuid), result);
    }
    /// attempt to get result of i32
    #[tracing::instrument(level = "debug", skip(self))]
    pub fn check_i32(&self, spliter: i32, uuid: &Uuid) -> Option<i32> {
        tracing::trace!(request_id=?uuid);
        #[cfg(feature = "single-instance")]
        if let Some(x) = self.dup_i32.get(&(spliter, *uuid)) {
            log::debug!("duplicated request_id: {}, result: {}", uuid, x);
            return Some(x);
        }
        None
    }
    /// attempt to get result of String
    #[tracing::instrument(level = "debug", skip(self))]
    pub fn check_str(&self, spliter: i32, uuid: &Uuid) -> Option<String> {
        tracing::trace!(request_id=?uuid);
        #[cfg(feature = "single-instance")]
        if let Some(x) = self.dup_str.get(&(spliter, *uuid)) {
            log::debug!("duplicated request_id: {}, result: {}", uuid, x);
            return Some(x);
        }
        None
    }
}
