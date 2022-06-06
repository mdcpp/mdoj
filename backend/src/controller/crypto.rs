use std::sync::{Arc, Mutex};

use crate::entity::{prelude::TokenTable as Token, token_table as token};
use lru::LruCache;
use openssl::symm::Mode;
use openssl::{aes, base64};
use rand::prelude::*;
use sea_orm::{DatabaseConnection, EntityTrait, Set};
use serde::{de::DeserializeOwned, Serialize};

const AES_KEY: &[u8; 32] = include_bytes!["../../config/aes"];

type SaltType = [u8; 32];
type CounterType = i32;

pub struct Cache {
    lru: LruCache<usize, SaltType>,
    counter: CounterType,
}

impl Cache {
    pub fn new() -> Self {
        Cache {
            lru: LruCache::new(100),
            counter: 0,
        }
    }
}

pub async fn encode<T>(payload: T, cache: &mut Cache, conn: &DatabaseConnection) -> String
where
    T: Serialize,
{
    let mut rng = rand::thread_rng();
    let salt: SaltType = rng.gen();
    // regist salt
    let id = cache.counter;
    Token::insert(token::ActiveModel {
        id: Set(id),
        salt: Set(salt.to_vec()),
    })
    .exec(conn)
    .await
    .unwrap();
    cache.counter = cache.counter + 1;
    // serialize payload
    let bytea = bincode::serialize(&payload).unwrap();
    let len = bincode::serialize(&(bytea.len() as u32)).unwrap();
    let id = bincode::serialize(&(id as CounterType)).unwrap();
    // fill bytea with random bytes (AES IGE require the input bytea to be multiple of 16)
    let mut rng = rand::thread_rng();
    let mut seed = rng.gen::<[u8; 32]>().to_vec();
    seed.truncate(16 - bytea.len() % 16 + 12);
    let bytea = [len, bytea, seed.to_owned()].concat();
    // encrypt aes256
    let key = aes::AesKey::new_encrypt(AES_KEY).unwrap();
    let mut output = vec![0u8; bytea.len()];
    aes::aes_ige(&bytea, &mut output, &key, &mut salt.clone(), Mode::Encrypt);
    // encrypt base64
    base64::encode_block(&[id, output].concat())
}

pub async fn decode<'a, T>(input: &str, cache: Cache, conn: &DatabaseConnection) -> Option<T>
where
    T: DeserializeOwned,
{
    match base64::decode_block(&input) {
        Ok(x) => {
            let id: CounterType = bincode::deserialize_from(&x[0..4]).unwrap();
            let h = Token::find_by_id(id).one(conn).await.unwrap();
            match h {
                Some(model) => {
                    let key = aes::AesKey::new_decrypt(AES_KEY).unwrap();
                    let salt = model.salt;
                    let mut output = vec![0_u8; x.len() - 4];
                    aes::aes_ige(
                        &x[4..(x.len() - 1)],
                        &mut output,
                        &key,
                        &mut salt.clone(),
                        Mode::Decrypt,
                    );
                    let offset: u32 = bincode::deserialize(&(output[0..4])).unwrap();
                    // let output: &'a Vec<u8>=&output;
                    match bincode::deserialize(&output[4..(offset as usize + 3)]) {
                        Ok(x) => Some(x),
                        Err(_) => None,
                    }
                }
                None => None,
            }
            // get salt by id
        }
        Err(_) => None,
    }
}

#[cfg(test)]
mod test {
    use super::*;
    // check if openssl panic
    // #[async_std::test]
    // #[ignore]
    // async fn generate_token(){
    //     #[derive(Serialize, Deserialize)]
    //     pub struct payload<'a> {
    //         pub username: &'a str,
    //         pub password: &'a str,
    //     }
    //     let data=payload{ username: "Lorem Ipsum", password: "readable variations" };
    //     let salt=b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C\x0D\x0E\x0F\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C\x0D\x0E\x0F";
    //     let token=encode(data,  &salt).await;
    //     dbg!(token);
    // }
}
