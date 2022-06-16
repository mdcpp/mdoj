use crate::entity::{prelude::TokenTable as Token, token_table as token};
use bincode;
use lru::LruCache;
use openssl::{aes, base64, symm::Mode};
use rand::prelude::*;
use sea_orm::{prelude::*, Set};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::sync::atomic::AtomicI32;
use std::{
    mem::size_of,
    sync::{atomic, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const TTL: i64 = 3600;
type TTLType = i64;
type RandomType = [u8; 32];
type TokenType = (i32, RandomType);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TokenData {
    user_pkey: i32,
}

pub struct TokenState<'a> {
    counter: AtomicI32,
    cache: Mutex<LruCache<(i32, RandomType), (TTLType, TokenData)>>,
    conn: &'a DatabaseConnection,
}

impl<'a> TokenState<'a> {
    pub fn new(conn: &'a DatabaseConnection) -> Self {
        TokenState {
            counter: AtomicI32::new(0),
            cache: Mutex::new(LruCache::new(1000)),
            conn: &conn,
        }
    }
}

fn get_time() -> TTLType {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as TTLType
}

pub async fn issue<'a>(state: &TokenState<'a>, data: TokenData) -> String {
    let now = get_time();

    let count = state.counter.fetch_add(1, atomic::Ordering::Relaxed);
    let mut rng = rand::thread_rng();
    let seed: RandomType = rng.gen();

    let payload = bincode::serialize(&data).unwrap();

    let count_in_bytes = bincode::serialize(&count).unwrap();

    let base64 = base64::encode_block(&[count_in_bytes, seed.to_vec()].concat());

    state
        .cache
        .lock()
        .unwrap()
        .put((count, seed), (now + TTL, data));

    Token::insert(token::ActiveModel {
        id: Set(count),
        key: Set(seed.to_vec()),
        data: Set(payload),
        ttl: Set(now + TTL),
    })
    .exec(state.conn)
    .await
    .unwrap();

    base64
}

pub async fn verify<'a>(state: &TokenState<'a>, token: &str) -> Option<TokenData> {
    let token = base64::decode_block(token).unwrap();
    let token: TokenType = bincode::deserialize(&token).unwrap();

    let mut cache = state.cache.lock().unwrap();
    let lru_result = cache.get(&token);

    let now = get_time();

    if let Some(x) = lru_result {
        if now + TTL > x.0 {
            return Some(x.1.clone());
        }

        return None;
    }

    let result = Token::find_by_id(token.0).one(state.conn).await.unwrap();

    if None == result {
        return None;
    }

    let result = result.unwrap();

    if result.key != token.1 {
        return None;
    }

    if now + TTL <= result.ttl {
        return None;
    }

    Some(bincode::deserialize(&result.data.as_slice()).unwrap())
}

#[cfg(test)]
mod test {
    use actix_web::rt::time::sleep;
    use sea_orm::{ConnectionTrait, Database, DbBackend, Schema};

    use super::*;
    use futures::future::join_all;
    use std::sync::Arc;
    use std::time;

    #[actix_web::test]
    async fn test1() {}
}
