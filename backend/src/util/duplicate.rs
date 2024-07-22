use super::error::Result;
use grpc::backend::*;
use quick_cache::sync::Cache;
use std::future::Future;
use tonic::Response;
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
        create_cache!($t, $ret, 16);
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
                    match &self.request_id{
                        Some(x) => [<$t CacheInstance>]
                            .get(Uuid::parse_str(&x)?, || f(self))
                            .await,
                        None=> f(self).await
                    }
                }
            }
        }
    };
}

create_cache!(PublishRequest, ());
create_cache!(JoinContestRequest, ());
create_cache!(RemoveRequest, ());
create_cache!(RejudgeRequest, ());

create_cache!(RefreshRequest, TokenInfo, 8);
create_cache!(LoginRequest, TokenInfo, 8);

create_cache!(CreateAnnouncementRequest, Id);
create_cache!(CreateChatRequest, Id);
create_cache!(CreateContestRequest, Id);
create_cache!(CreateEducationRequest, Id);
create_cache!(CreateProblemRequest, Id);
create_cache!(CreateSubmitRequest, Id);
create_cache!(CreateTestcaseRequest, Id);
create_cache!(CreateUserRequest, Id);

create_cache!(UpdateAnnouncementRequest, ());
create_cache!(UpdateContestRequest, ());
create_cache!(UpdateEducationRequest, ());
create_cache!(UpdateProblemRequest, ());
create_cache!(UpdateTestcaseRequest, ());
create_cache!(UpdateUserRequest, ());
create_cache!(UpdatePasswordRequest, ());

create_cache!(AddAnnouncementToContestRequest, ());
create_cache!(AddEducationToProblemRequest, ());
create_cache!(UploadRequest, UploadResponse);
create_cache!(AddTestcaseToProblemRequest, ());
create_cache!(AddProblemToContestRequest, ());
