use crate::entity::{prelude::TokenTable as Token, token_table as token};
use lru::LruCache;
use openssl::symm::Mode;
use openssl::{aes, base64};
use rand::prelude::*;
use sea_orm::{DatabaseConnection, EntityTrait, Set};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

// Initialization vector:iv
const AES_KEY: &[u8; 32] = include_bytes!["../../config/aes"];

type InitVecType = [u8; 32];
type CounterType = i32;
// token::Model::id;

pub struct Cache {
    lru: LruCache<usize, InitVecType>,
    counter: CounterType,
}

impl Cache {
    pub fn new() -> Self {
        Cache {
            lru: LruCache::new(100),
            counter: 0,
        }
    }
    async fn insert(&mut self, val: InitVecType, conn: &DatabaseConnection) -> CounterType {
        let id = self.counter;
        Token::insert(token::ActiveModel {
            id: Set(id),
            salt: Set(val.to_vec()),
        })
        .exec(conn)
        .await
        .unwrap();
        self.counter = self.counter + 1;
        id
    }
    async fn retrieve(
        &mut self,
        key: CounterType,
        conn: &DatabaseConnection,
    ) -> Option<InitVecType> {
        match Token::find_by_id(key).one(conn).await.unwrap() {
            Some(x) => {
                let init_vec: InitVecType = x.salt.try_into().unwrap_or_else(|_| {
                    panic!("expect a vector length to be 32(length of init_vec)")
                });
                Some(init_vec)
            }
            None => None,
        }
    }
    async fn delete() {
        todo!();
    }
}

async fn encode<T>(payload: T, cache: &mut Cache, conn: &DatabaseConnection) -> String
where
    T: Serialize,
{
    // generate initialization vector
    let mut rng = rand::thread_rng();
    let init_vec: InitVecType = rng.gen();

    // serialize both "initialization vector id"(unencrypted) and "T"(encrypted)(named "bytea" here)
    let id = cache.insert(init_vec, conn).await;
    let bytea = bincode::serialize(&payload).unwrap();
    let len = bincode::serialize(&(bytea.len() as u32)).unwrap();
    let id = bincode::serialize(&(id as CounterType)).unwrap();
    let mut seed = rng.gen::<[u8; 32]>().to_vec();
    seed.truncate(16 - bytea.len() % 16 + 12);
    let bytea = [len, bytea, seed.to_owned()].concat();

    // encrypt aes256
    let key = aes::AesKey::new_encrypt(AES_KEY).unwrap();
    let mut output = vec![0u8; bytea.len()];
    aes::aes_ige(
        &bytea,
        &mut output,
        &key,
        &mut init_vec.clone(),
        Mode::Encrypt,
    );

    // encrypt base64
    base64::encode_block(&[id, output].concat())
}

async fn decode<T>(input: &str, cache: &mut Cache, conn: &DatabaseConnection) -> Option<T>
where
    T: DeserializeOwned,
{
    match base64::decode_block(&input) {
        Ok(x) => {
            // get id
            let id: CounterType = bincode::deserialize_from(&x[0..4]).unwrap();
            // get initialization vector by id
            let init_vec = cache.retrieve(id, conn).await;
            if init_vec == None {
                return None;
            }
            let init_vec = init_vec.unwrap();
            // serialize both "initialization vector id"(unencrypted) and "T"(encrypted)(named "bytea" here)
            let key = aes::AesKey::new_decrypt(AES_KEY).unwrap();
            let mut output = vec![0_u8; x.len() - 4];
            aes::aes_ige(
                &x[4..(x.len() - 1)],
                &mut output,
                &key,
                &mut init_vec.clone(),
                Mode::Decrypt,
            );
            let offset: u32 = bincode::deserialize(&(output[0..4])).unwrap();
            match bincode::deserialize(&output[4..(offset as usize + 3)]) {
                Ok(x) => Some(x),
                Err(_) => None,
            }
        }
        Err(_) => None,
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Otoken {
    role: u32,
}
impl Otoken {
    pub async fn encrypt<T>(&self, conn: &DatabaseConnection, cache: &mut Cache) -> String
    where
        T: Serialize,
    {
        encode(self, cache, conn).await
    }
    pub async fn decrypt<T>(inp: &str, conn: &DatabaseConnection, cache: &mut Cache) -> Option<T>
    where
        T: DeserializeOwned,
    {
        decode(inp, cache, conn).await
    }
    pub async fn revoke() {
        todo!();
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
    //     let init_vec=b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C\x0D\x0E\x0F\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C\x0D\x0E\x0F";
    //     let token=encode(data,  &init_vec).await;
    //     dbg!(token);
    // }

    // How to setup mock database which contain random value?
}
