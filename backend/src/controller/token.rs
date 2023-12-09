use chrono::{Duration, Local, NaiveDateTime};
use entity::token;
use opentelemetry::{global, metrics::ObservableGauge};
use quick_cache::sync::Cache;
use ring::rand::*;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseTransaction, EntityTrait, QueryFilter,
};
use std::sync::Arc;
use tokio::time;
use tracing::{instrument, Instrument, Span};

use crate::{
    init::{db::DB, logger::PACKAGE_NAME},
    report_internal,
};

const CLEAN_DUR: time::Duration = time::Duration::from_secs(60 * 30);
type Rand = [u8; 20];

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
                tonic::Status::invalid_argument("invaild token")
            }
            Error::Base64(_) => tonic::Status::invalid_argument("token should be base64"),
        }
    }
}

#[derive(Clone)]
struct CachedToken {
    user_id: i32,
    permission: u32,
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
    cache: Cache<Rand, CachedToken>,
    rand: SystemRandom,
    cache_meter: ObservableGauge<u64>,
}

impl TokenController {
    #[tracing::instrument(parent = span,name="token_construct_controller",level = "info",skip_all)]
    pub fn new(span: &Span) -> Arc<Self> {
        log::debug!("Setup TokenController");
        #[cfg(feature = "single-instance")]
        let cache = Cache::new(500);
        let self_ = Arc::new(Self {
            #[cfg(feature = "single-instance")]
            cache,
            rand: SystemRandom::new(),
            cache_meter: global::meter(PACKAGE_NAME)
                .u64_observable_gauge("cached_token")
                .init(),
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
    #[instrument(skip_all, name="token_create_controller",level="debug",fields(user = user.id))]
    pub async fn add(
        &self,
        user: &entity::user::Model,
        dur: Duration,
    ) -> Result<(String, NaiveDateTime), Error> {
        let db = DB.get().unwrap();

        let rand = generate(&self.rand).unwrap();
        let rand: Rand = rand.expose();

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

        Ok((
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD_NO_PAD, rand),
            expiry,
        ))
    }

    #[instrument(skip_all, name = "token_verify_controller", level = "debug")]
    pub async fn verify(&self, token: &str) -> Result<(i32, UserPermBytes), Error> {
        let now = Local::now().naive_local();
        let db = DB.get().unwrap();

        let rand =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD_NO_PAD, token)?;
        let rand: Rand = rand.try_into().map_err(|_| Error::InvalidTokenLength)?;

        let token: CachedToken;

        #[cfg(feature = "single-instance")]
        let cache_result = {
            match self.cache.get(&rand) {
                Some(cc) => {
                    if cc.expiry < now {
                        self.cache.remove(&rand);
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

        let token = match cache_result {
            Some(token) => {
                tracing::trace!(user_id = token.user_id, "cache_hit");
                token
            }
            None => {
                token = (token::Entity::find()
                    .filter(token::Column::Rand.eq(rand.to_vec()))
                    .one(db)
                    .in_current_span()
                    .await?
                    .ok_or(Error::NonExist)?)
                .into();

                tracing::trace!(user_id = token.user_id, "cache_missed");

                #[cfg(feature = "single-instance")]
                {
                    self.cache.insert(rand, token.clone());
                    self.cache_meter.observe(self.cache.weight(), &[]);
                }

                token
            }
        };

        if token.expiry < now {
            tracing::debug!(user_id = token.user_id, "token expired");
            return Err(Error::Expired);
        }

        Ok((token.user_id, UserPermBytes(token.permission)))
    }
    #[instrument(skip_all, name="token_remove_controller",level="debug", fields(token = token))]
    pub async fn remove(&self, token: String) -> Result<Option<()>, Error> {
        let db = DB.get().unwrap();

        let rand =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD_NO_PAD, token)?;
        let rand: Rand = rand.try_into().map_err(|_| Error::InvalidTokenLength)?;

        token::Entity::delete_many()
            .filter(token::Column::Rand.eq(rand.to_vec()))
            .exec(db)
            .await?;

        #[cfg(feature = "single-instance")]
        self.cache.remove(&rand);

        Ok(Some(()))
    }
    #[instrument(skip_all, name="token_removal",level="debug", fields(uid = user_id))]
    pub async fn remove_by_user_id(
        &self,
        user_id: i32,
        txn: &DatabaseTransaction,
    ) -> Result<(), Error> {
        let models = token::Entity::find()
            .filter(token::Column::UserId.eq(user_id))
            .all(txn)
            .await?;

        for model in models {
            self.cache.remove(model.rand.as_slice());
        }
        token::Entity::delete_many()
            .filter(token::Column::UserId.eq(user_id))
            .exec(txn)
            .await?;

        Ok(())
    }
}

macro_rules! set_bit_value {
    ($item:ident,$name:ident,$pos:expr) => {
        paste::paste! {
            impl $item{
                pub fn [<can_ $name>](&self)->bool{
                    let filter = 1_u32<<($pos);
                    (self.0&filter) == filter
                }
                pub fn [<grant_ $name>](&mut self,value:bool){
                    let filter = 1_u32<<($pos);
                    if (self.0&filter == filter) ^ value{
                        self.0 ^= filter;
                    }
                }
            }
        }
    };
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct UserPermBytes(pub u32);

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
set_bit_value!(UserPermBytes, manage_user, 8);
