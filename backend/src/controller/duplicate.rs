use quick_cache::sync::Cache;
use std::future::Future;
use std::sync::Arc;
use std::{
    any::{Any, TypeId},
    ops::Deref,
};
use tracing::Span;
use uuid::Uuid;

#[derive(Eq, Hash, PartialEq)]
struct DupKey {
    user_id: i32,
    request_id: Uuid,
    type_id: TypeId,
}

/// Request Duplication
///
/// It cache request result with fat pointer and provide safe interface to access data
///
/// Note that for effeciency, it uses Clock-Pro cache algorithm, expect occasional missing,
/// shouldn't be rely on in unstable connection
pub struct DupController {
    store: Cache<DupKey, Arc<dyn Any + 'static + Send + Sync>>,
}

impl DupController {
    #[tracing::instrument(parent=span, name="duplicate_construct",level = "info",skip_all)]
    pub fn new(span: &Span) -> Self {
        Self {
            store: Cache::new(128),
        }
    }
    /// store request_id and result
    #[tracing::instrument(name = "controller.duplicate.store", level = "debug", skip_all)]
    pub fn store<T>(&self, user_id: i32, request_id: Uuid, result: T)
    where
        T: 'static + Send + Sync + Clone,
    {
        let key = DupKey {
            user_id,
            request_id,
            type_id: result.type_id(),
        };
        self.store.insert(key, Arc::new(result));
    }
    /// check request_id and result
    #[tracing::instrument(name = "controller.duplicate.get", level = "debug", skip_all)]
    pub fn check<T>(&self, user_id: i32, request_id: Uuid) -> Option<T>
    where
        T: 'static + Send + Sync + Clone,
    {
        let key = DupKey {
            user_id,
            request_id,
            type_id: TypeId::of::<T>(),
        };
        self.store
            .peek(&key)
            .map(|x| x.deref().downcast_ref::<T>().unwrap().clone())
    }
}
