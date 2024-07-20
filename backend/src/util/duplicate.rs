use super::error::Result;
use grpc::backend::*;
use quick_cache::sync::Cache;
use std::future::Future;
use uuid::Uuid;

/// caching singleton trait
///
/// In addition to caching, it also includes error_handling and async support.
pub trait Cacheable
where
    Self: Sized,
{
    type Item: 'static + Send + Sync + Clone;
    fn get_or_insert<F, Fut>(self, f: F) -> impl Future<Output = Result<Self::Item>>
    where
        F: FnOnce(Self) -> Fut,
        Fut: Future<Output = Result<Self::Item>>;
    fn process<F, Fut>(
        self,
        f: F,
    ) -> impl Future<Output = tonic::Result<tonic::Response<Self::Item>>>
    where
        F: FnOnce(Self) -> Fut,
        Fut: Future<Output = Result<Self::Item>>,
    {
        async move {
            match self.get_or_insert(f).await {
                Ok(x) => Ok(tonic::Response::new(x)),
                Err(err) => Err(err.into()),
            }
        }
    }
}

/// implement [`Cacheable`] for a type
///
/// Example:
/// ```rust
/// struct MyRequest{
///     // this field is used as key for caching
///     request_id: String
/// }
/// struct MyResponse;
/// create_cache!(MyRequest, MyResponse, 32);// cache with capacity of 32
///
/// fn main(){
///     let req = MyReuqest{request_id: "eefd5403-52f4-4f9e-92c5-8e85ae16733b".to_owned()};
///     let res = req.get_or_insert(|req| async {Ok(MyResponse}).await?;
/// }
/// ```
macro_rules! create_cache {
    ($t:ident, $ret:ty) => {
        create_cache!($t, $ret, 32);
    };
    ($t:ident, $ret:ty, $cap:expr) => {
        paste::paste! {
            struct [<$t Cache>] {
                cache: Cache<Uuid, $ret>,
            }
            lazy_static::lazy_static! {
                static ref [<$t CacheInstance>]: [<$t Cache>]=[<$t Cache>]::new();
            }
            impl [<$t Cache>] {
                fn new() -> Self {
                    Self {
                        cache: Cache::new($cap),
                    }
                }
                async fn get<F, Fut>(&self, uuid: Uuid, f: F) -> Result<$ret>
                where
                    F: FnOnce() -> Fut,
                    Fut: Future<Output = Result<$ret>>,
                {
                    if let Some(x) = self.cache.peek(&uuid) {
                        return Ok(x.clone());
                    }
                    let res = f().await?;
                    self.cache.insert(uuid, res.clone());
                    Ok(res)
                }
            }
            impl Cacheable for $t {
                type Item = $ret;
                async fn get_or_insert<F, Fut>(self, f: F) -> Result<Self::Item>
                where
                    F: FnOnce(Self) -> Fut,
                    Fut: Future<Output = Result<Self::Item>>,
                {
                    let uuid = Uuid::parse_str(&self.request_id)?;
                    [<$t CacheInstance>]
                        .get(uuid, || f(self))
                        .await
                }
            }
        }
    };
}

create_cache!(CreateAnnouncementRequest, Id);
