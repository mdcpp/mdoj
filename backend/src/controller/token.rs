use crate::entity::{prelude::TokenTable as Token, token_table as token};
use lru::LruCache;
use openssl::{aes, base64, symm::Mode};
use rand::prelude::*;
use sea_orm::prelude::*;
use sea_orm::Set;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::sync::Mutex;

// Initialization vector:iv
const AES_KEY: &[u8; 32] = include_bytes!["../../config/aes"];

type InitVecType = [u8; 32];
type CounterType = i32;
// token::Model::id;

#[derive(Debug)]
pub struct Cache {
    lru: Mutex<LruCache<CounterType, InitVecType>>,
    counter: Mutex<CounterType>,
}

impl Cache {
    pub fn new(cap: usize) -> Self {
        Cache {
            lru: Mutex::new(LruCache::new(cap)),
            counter: Mutex::new(0),
        }
    }
    async fn insert(&self, val: InitVecType, conn: &DatabaseConnection) -> CounterType {
        // get id and put into cache
        let mut lrug = self.lru.lock().unwrap();
        let mut idg = self.counter.lock().unwrap();
        *idg = *idg + 1;
        let id = (*idg).clone();
        lrug.put(id, val);
        drop(lrug);
        drop(idg);

        Token::insert(token::ActiveModel {
            id: Set(id),
            salt: Set(val.to_vec()),
        })
        .exec(conn)
        .await
        .unwrap();
        id
    }
    async fn retrieve(&self, key: CounterType, conn: &DatabaseConnection) -> Option<InitVecType> {
        let mut lrug = self.lru.lock().unwrap();
        let result = lrug.get(&key);
        if result.is_some() {
            return Some(result.unwrap().clone());
        }
        drop(lrug);
        match Token::find_by_id(key).one(conn).await.unwrap() {
            Some(x) => {
                let init_vec: InitVecType = x.salt.try_into().unwrap_or_else(|_| {
                    panic!("expect a vector length to be 32(length of init_vec)")
                });
                let mut lrug = self.lru.lock().unwrap();
                lrug.put(key, init_vec);
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
    use actix_web::rt::time::sleep;
    use sea_orm::{ConnectionTrait, Database, DatabaseBackend, DbBackend, MockDatabase, Schema};

    use super::*;
    use futures::future::join_all;
    use std::sync::Arc;
    use std::time;

    // #[actix_rt::test] 
    #[actix_web::test]
    // #[async_std::test]
    async fn cache_test() {
        impl Clone for AppState {
            fn clone(&self) -> Self {
                AppState {
                    conn: self.conn.clone(),
                    cache: self.cache.clone(),
                }
            }
        }
        // let db: DatabaseConnection = MockDatabase::new(DatabaseBackend::Postgres).into_connection();
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let schema = Schema::new(DbBackend::Sqlite);

        let stmt = schema.create_table_from_entity(Token);
        db.execute(db.get_database_backend().build(&stmt))
            .await
            .unwrap();

        struct AppState {
            conn: Arc<DatabaseConnection>,
            cache: Arc<Cache>,
        }
        let state = AppState {
            conn: Arc::new(db),
            cache: Arc::new(Cache::new(100)),
        };

        async fn spawn_one_thread(state: AppState) -> Option<bool> {
            let mut rng = rand::thread_rng();
            let random: InitVecType = rng.gen();

            let key = state.cache.insert(random, &(*state.conn)).await;
            sleep(time::Duration::from_millis(10)).await;
            let random_output = state.cache.retrieve(key, &(*state.conn)).await;
            match random_output {
                Some(x) => Some(x == random),
                None => None,
            }
        }

        let mut promises = Vec::new();

        for _ in 0..100 {
            promises.push(spawn_one_thread(state.clone()));
        }

        let result = join_all(promises).await;

        dbg!(result);
    }
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
