use chrono::{Duration, Local, NaiveDateTime};
use entity::token;
use lru::LruCache;
use ring::rand::*;
use sea_orm::{ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter};
use spin::mutex::Mutex;
use std::num::NonZeroUsize;
use std::sync::Arc;
use tokio::time;
use tracing::instrument;

use crate::init::db::DB;

use super::Error;

const CLEAN_DUR: time::Duration = time::Duration::from_secs(60 * 30);
type RAND = [u8; 20];

macro_rules! report {
    ($e:expr) => {
        match $e {
            Some(x) => x,
            None => {
                return Ok(None);
            }
        }
    };
}

#[derive(Clone)]
struct CachedToken {
    user_id: i32,
    permission: u64,
    expiry: NaiveDateTime,
}

impl From<token::Model> for CachedToken {
    fn from(value: token::Model) -> Self {
        Self {
            user_id: value.user_id,
            permission: value.permission,
            expiry: value.expiry,
        }
    }
}

pub struct TokenController {
    #[cfg(feature = "single-instance")]
    cache: Mutex<LruCache<RAND, CachedToken>>,
    rand: SystemRandom,
    // reverse_proxy:Arc<RwLock<BTreeSet<IpAddr>>>,
}

impl TokenController {
    pub fn new() -> Arc<Self> {
        log::debug!("Setup TokenController");
        #[cfg(feature = "single-instance")]
        let cache = Mutex::new(LruCache::new(NonZeroUsize::new(100).unwrap()));
        let self_ = Arc::new(Self {
            #[cfg(feature = "single-instance")]
            cache,
            rand: SystemRandom::new(),
        });
        tokio::spawn(async move {
            let db = DB.get().unwrap();
            loop {
                time::sleep(CLEAN_DUR).await;
                let now = Local::now().naive_local();

                if let Err(err) = token::Entity::delete_many()
                    .filter(token::Column::Expiry.lte(now))
                    .exec(db)
                    .await
                {
                    log::error!("Token clean failed: {}", err);
                }
            }
        });
        self_
    }
    #[instrument(skip_all, name="token_create",fields(user = user.id))]
    pub async fn add(
        &self,
        user: &entity::user::Model,
        dur: Duration,
    ) -> Result<(String, NaiveDateTime), Error> {
        let db = DB.get().unwrap();

        let rand = generate(&self.rand).unwrap();
        let rand: RAND = rand.expose();

        let expiry = (Local::now() + dur).naive_local();

        token::ActiveModel {
            user_id: ActiveValue::Set(user.id),
            rand: ActiveValue::Set(rand.to_vec().clone()),
            expiry: ActiveValue::Set(expiry),
            permission: ActiveValue::Set(user.permission),
            ..Default::default()
        }
        .insert(db)
        .await?;

        Ok((hex::encode(rand), expiry))
    }
    // pub async fn verify_throttle(&self, token:&str, ip:Option<IpAddr>)-> Result<Option<(i32, UserPermBytes)>, Error>{
    //     let reverse_proxy=self.reverse_proxy.read();
    //     if reverse_proxy.len()==0{
    //         return  self.verify(token).await;
    //     }else{
    //         if let Some(ip)=ip{
    //             if !reverse_proxy.contains(&ip){
    //                 return self.verify(token).await;
    //             }
    //         }
    //     }
    //     return Ok(None);
    // }
    // #[instrument(skip_all, name = "token_verify")]
    pub async fn verify(&self, token: &str) -> Result<Option<(i32, UserPermBytes)>, Error> {
        let now = Local::now().naive_local();
        let db = DB.get().unwrap();

        let rand = report!(hex::decode(token).ok());
        let rand: RAND = report!(rand.try_into().ok());

        let token: CachedToken;

        #[cfg(feature = "single-instance")]
        let cache_result = {
            let mut cache = self.cache.lock();
            match cache.get(&rand) {
                Some(cc) => {
                    if cc.expiry < now {
                        cache.pop(&rand);
                        None
                    } else {
                        Some(cc.clone())
                    }
                }
                None => None,
            }
        };
        #[cfg(not(feature = "single-instance"))]
        let cache_result: Option<CachedToken> = None;

        match cache_result {
            Some(token_) => {
                token = token_;
            }
            None => {
                token = report!(
                    token::Entity::find()
                        .filter(token::Column::Rand.eq(rand.to_vec()))
                        .one(db)
                        .await?
                )
                .into();

                if token.expiry < now {
                    return Ok(None);
                }

                #[cfg(feature = "single-instance")]
                self.cache.lock().put(rand, token.clone());
            }
        }

        Ok(Some((token.user_id, UserPermBytes(token.permission))))
    }
    #[instrument(skip_all, name="token_removal", fields(token = token))]
    pub async fn remove(&self, token: String) -> Result<Option<()>, Error> {
        let db = DB.get().unwrap();

        let rand = report!(hex::decode(token).ok());
        let rand: RAND = report!(rand.try_into().ok());

        token::Entity::delete_many()
            .filter(token::Column::Rand.eq(rand.to_vec()))
            .exec(db)
            .await?;

        #[cfg(feature = "single-instance")]
        self.cache.lock().pop(&rand);

        Ok(Some(()))
    }
}

macro_rules! set_bit_value {
    ($item:ident,$name:ident,$pos:expr) => {
        paste::paste! {
            impl $item{
                pub fn [<can_ $name>](&self)->bool{
                    let filter = 1_u64<<($pos);
                    (self.0&filter) == filter
                }
                pub fn [<grant_ $name>](&mut self,value:bool){
                    let filter = 1_u64<<($pos);
                    if (self.0&filter == filter) ^ value{
                        self.0 = self.0 ^ filter;
                    }
                }
            }
        }
    };
}

#[derive(Debug, Clone, Copy)]
pub struct UserPermBytes(pub u64);

impl UserPermBytes {
    pub fn strict_ge(&self, other: Self) -> bool {
        (self.0 | other.0) == other.0
    }
}

set_bit_value!(UserPermBytes, root, 0);
set_bit_value!(UserPermBytes, manage_problem, 1);
set_bit_value!(UserPermBytes, manage_education, 2);
set_bit_value!(UserPermBytes, manage_announcement, 3);
set_bit_value!(UserPermBytes, manage_submit, 4);
set_bit_value!(UserPermBytes, publish, 5);
set_bit_value!(UserPermBytes, link, 6);
set_bit_value!(UserPermBytes, manage_contest, 7);
