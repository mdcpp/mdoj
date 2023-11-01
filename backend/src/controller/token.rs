use chrono::{Duration, Local, NaiveDateTime};
use entity::token;
use lru::LruCache;
use rand::Rng;
use sea_orm::{ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter};
use spin::mutex::spin::SpinMutex;
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::common::prelude::UserPermBytes;
use crate::init::db::DB;

use super::Error;

const EXPIRY_FRQU: usize = 10;
type RAND = [u8; 16];

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
    permission: i64,
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
    cache: SpinMutex<LruCache<RAND, CachedToken>>,
    frqu: AtomicUsize,
}

impl Default for TokenController {
    fn default() -> Self {
        log::debug!("Setup TokenController");
        let cache = SpinMutex::new(LruCache::new(NonZeroUsize::new(100).unwrap()));
        Self {
            cache,
            frqu: Default::default(),
        }
    }
}

impl TokenController {
    pub async fn add(&self, user: &entity::user::Model, dur: Duration) -> Result<String, Error> {
        let db = DB.get().unwrap();

        let mut rng = rand::thread_rng();
        let rand: i128 = rng.gen();
        let rand: RAND = rand.to_be_bytes();

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

        Ok(hex::encode(rand))
    }
    pub async fn verify(&self, token: &str) -> Result<Option<(i32, UserPermBytes)>, Error> {
        let now = Local::now().naive_local();
        let db = DB.get().unwrap();

        if self.frqu.fetch_add(1, Ordering::Relaxed) % EXPIRY_FRQU == 1 {
            tokio::spawn(
                token::Entity::delete_many()
                    .filter(token::Column::Expiry.lte(now))
                    .exec(db),
            );
        }

        let rand = report!(hex::decode(token).ok());
        let rand: [u8; 16] = report!(rand.try_into().ok());

        let token: CachedToken;

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

                self.cache.lock().put(rand, token.clone());
            }
        }

        Ok(Some((token.user_id, UserPermBytes(token.permission))))
    }
    pub async fn remove(&self, token: String) -> Result<Option<()>, Error> {
        let db = DB.get().unwrap();

        let rand = report!(hex::decode(token).ok());
        let rand: [u8; 16] = report!(rand.try_into().ok());

        token::Entity::delete_many()
            .filter(token::Column::Rand.eq(rand.to_vec()))
            .exec(db)
            .await?;

        self.cache.lock().pop(&rand);

        Ok(Some(()))
    }
}
