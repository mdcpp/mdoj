use crate::{entity::token, util::auth::RoleLv};
use chrono::{Duration, Local, NaiveDateTime};
use quick_cache::sync::Cache;
use rand::{Rng, SeedableRng};
use rand_hc::Hc128Rng;
use sea_orm::*;
use spin::Mutex;
use std::{ops::Deref, sync::Arc};
use tokio::time;
use tracing::{instrument, Instrument};

use crate::report_internal;

/// cache size for main pool(vaildated list)
const CACHE_SIZE: usize = 419000; // about 16MiB
/// interval for database clean up
const CLEAN_DUR: time::Duration = time::Duration::from_secs(60 * 30);
/// len of token
const TOKEN_SIZE: usize = 20;
type Rand = [u8; TOKEN_SIZE];

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("`{0}`")]
    Database(#[from] sea_orm::error::DbErr),
    #[error("expired")]
    Expired,
    #[error("length of token is not 20")]
    InvalidTokenLength,
    #[error("`{0}`")]
    Base64(#[from] base64::DecodeError),
    #[error("token not exist")]
    NonExist,
}

impl From<Error> for tonic::Status {
    fn from(value: Error) -> Self {
        match value {
            Error::Database(x) => report_internal!(error, "`{}`", x),
            Error::NonExist | Error::Expired | Error::InvalidTokenLength => {
                tonic::Status::unauthenticated("invaild token")
            }
            Error::Base64(_) => tonic::Status::invalid_argument("token should be base64"),
        }
    }
}

#[derive(Clone)]
struct CachedToken {
    user_id: i32,
    permission: i32,
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
    cache: Cache<Rand, CachedToken>,
    rng: Mutex<Hc128Rng>,
    db: Arc<DatabaseConnection>,
}

impl TokenController {
    #[tracing::instrument(name = "token_construct_controller", level = "info", skip_all)]
    pub fn new(db: Arc<DatabaseConnection>) -> Arc<Self> {
        log::debug!("Setup TokenController");
        let cache = Cache::new(CACHE_SIZE);
        let self_ = Arc::new(Self {
            cache,
            rng: Mutex::new(Hc128Rng::from_entropy()),
            db: db.clone(),
        });
        tokio::spawn(async move {
            loop {
                time::sleep(CLEAN_DUR).await;
                let now = Local::now().naive_local();

                if let Err(err) = token::Entity::delete_many()
                    .filter(token::Column::Expiry.lte(now))
                    .exec(db.deref())
                    .await
                {
                    log::error!("Token clean failed: {}", err);
                }
            }
        });
        self_
    }
    #[instrument(skip_all, name="token_create_controller",level="debug",fields(user = user.id))]
    pub async fn add(
        &self,
        user: &crate::entity::user::Model,
        dur: Duration,
    ) -> Result<(String, NaiveDateTime), Error> {
        let rand: Rand = { self.rng.lock().gen() };

        let expiry = (Local::now() + dur).naive_local();

        token::ActiveModel {
            user_id: ActiveValue::Set(user.id),
            rand: ActiveValue::Set(rand.to_vec().clone()),
            expiry: ActiveValue::Set(expiry),
            permission: ActiveValue::Set(user.permission),
            ..Default::default()
        }
        .insert(self.db.deref())
        .in_current_span()
        .await?;

        Ok((
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD_NO_PAD, rand),
            expiry,
        ))
    }
    #[instrument(skip_all, name = "token_verify_controller", level = "trace")]
    pub async fn verify(&self, token: &str) -> Result<(i32, RoleLv), Error> {
        // FIXME: we need to cache hashed password, it's better to do that without coupling with user creation
        let now = Local::now().naive_local();

        let rand =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD_NO_PAD, token)?;
        let rand: Rand = rand.try_into().map_err(|_| Error::InvalidTokenLength)?;

        let cache_result = match self.cache.get(&rand) {
            Some(cc) => {
                if cc.expiry < now {
                    self.cache.remove(&rand);
                    None
                } else {
                    Some(cc.clone())
                }
            }
            None => None,
        };

        let token = match cache_result {
            Some(token) => {
                tracing::trace!(user_id = token.user_id, "cache_hit");
                token
            }
            None => {
                let token: CachedToken = (token::Entity::find()
                    .filter(token::Column::Rand.eq(rand.to_vec()))
                    .one(self.db.deref())
                    .in_current_span()
                    .await?
                    .ok_or(Error::NonExist)?)
                .into();

                tracing::trace!(user_id = token.user_id, "cache_missed");

                self.cache.insert(rand, token.clone());

                token
            }
        };

        if token.expiry < now {
            tracing::debug!(user_id = token.user_id, "token expired");
            return Err(Error::Expired);
        }

        Ok((token.user_id, token.permission.try_into().unwrap()))
    }
    #[instrument(skip_all, name="token_remove_controller",level="debug", fields(token = token))]
    pub async fn remove(&self, token: String) -> Result<Option<()>, Error> {
        let rand =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD_NO_PAD, token)?;
        let rand: Rand = rand.try_into().map_err(|_| Error::InvalidTokenLength)?;

        token::Entity::delete_many()
            .filter(token::Column::Rand.eq(rand.to_vec()))
            .exec(self.db.deref())
            .await?;

        self.cache.remove(&rand);

        Ok(Some(()))
    }
    /// remove user's token by user id
    #[instrument(skip_all, name="token_removal",level="debug", fields(uid = user_id))]
    pub async fn remove_by_user_id(
        &self,
        user_id: i32,
        // txn: &DatabaseTransaction,
    ) -> Result<(), Error> {
        let db = self.db.deref();

        let models = token::Entity::find()
            .filter(token::Column::UserId.eq(user_id))
            .all(db)
            .await?;

        for model in models {
            self.cache.remove(model.rand.as_slice());
        }
        token::Entity::delete_many()
            .filter(token::Column::UserId.eq(user_id))
            .exec(db)
            .await?;

        Ok(())
    }
}
