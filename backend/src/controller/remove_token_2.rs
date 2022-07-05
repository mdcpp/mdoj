// use entity::{prelude::TokenTable as Token, token_table as token};
// use lru::LruCache;
// use openssl::{aes, base64, symm::Mode};
// use rand::prelude::*;
// use sea_orm::prelude::*;
// use sea_orm::Set;
// use serde::{de::DeserializeOwned, Deserialize, Serialize};
// use std::sync::Mutex;

// // Initialization vector:iv
// const AES_KEY: &[u8; 32] = include_bytes!["../../config/aes"];

// type InitVecType = [u8; 32];
// type CounterType = i32;
// // token::Model::id;

// #[derive(Debug)]
// pub struct Cache {
//     lru: Mutex<LruCache<CounterType, InitVecType>>,
//     counter: Mutex<CounterType>,
// }

// impl Cache {
//     pub fn new(cap: usize) -> Self {
//         Cache {
//             lru: Mutex::new(LruCache::new(cap)),
//             counter: Mutex::new(0),
//         }
//     }
//     async fn insert(&self, val: InitVecType, conn: &DatabaseConnection) -> CounterType {
//         // get id and put into cache
//         let mut lrug = self.lru.lock().unwrap();
//         let mut idg = self.counter.lock().unwrap();
//         *idg = *idg + 1;
//         let id = (*idg).clone();
//         lrug.put(id, val);
//         drop(lrug);
//         drop(idg);

//         Token::insert(token::ActiveModel {
//             id: Set(id),
//             salt: Set(val.to_vec()),
//         })
//         .exec(conn)
//         .await
//         .unwrap();
//         id
//     }
//     async fn retrieve(&self, key: CounterType, conn: &DatabaseConnection) -> Option<InitVecType> {
//         let mut lrug = self.lru.lock().unwrap();
//         let result = lrug.get(&key);
//         if result.is_some() {
//             return Some(result.unwrap().clone());
//         }
//         drop(lrug);
//         match Token::find_by_id(key).one(conn).await.unwrap() {
//             Some(x) => {
//                 let init_vec: InitVecType = x.salt.try_into().unwrap_or_else(|_| {
//                     panic!("expect a vector length to be 32(length of init_vec)")
//                 });
//                 let mut lrug = self.lru.lock().unwrap();
//                 lrug.put(key, init_vec);
//                 Some(init_vec)
//             }
//             None => None,
//         }
//     }
//     async fn delete() {
//         todo!();
//     }
// }

// async fn encode<T>(payload: T, cache: &Cache, conn: &DatabaseConnection) -> String
// where
//     T: Serialize,
// {
//     // generate initialization vector
//     let mut rng = rand::thread_rng();
//     let init_vec: InitVecType = rng.gen();

//     // serialize both "initialization vector id"(unencrypted) and "T"(encrypted)(named "bytea" here)
//     let id = cache.insert(init_vec, conn).await;
//     let bytea = bincode::serialize(&payload).unwrap();
//     let len = bincode::serialize(&(bytea.len() as u32)).unwrap();
//     let id = bincode::serialize(&(id as CounterType)).unwrap();
//     let mut seed = rng.gen::<[u8; 32]>().to_vec();
//     seed.truncate(16 - bytea.len() % 16 + 12);
//     let bytea = [len, bytea, seed.to_owned()].concat();

//     // encrypt aes256
//     let key = aes::AesKey::new_encrypt(AES_KEY).unwrap();
//     let mut output = vec![0u8; bytea.len()];
//     aes::aes_ige(
//         &bytea,
//         &mut output,
//         &key,
//         &mut init_vec.clone(),
//         Mode::Encrypt,
//     );

//     // encrypt base64
//     base64::encode_block(&[id, output].concat())
// }

// async fn decode<T>(input: &str, cache: & Cache, conn: &DatabaseConnection) -> Option<T>
// where
//     T: DeserializeOwned,
// {
//     match base64::decode_block(&input) {
//         Ok(x) => {
//             // get id
//             let id: CounterType = bincode::deserialize_from(&x[0..4]).unwrap();
//             // get initialization vector by id
//             let init_vec = cache.retrieve(id, conn).await;
//             if init_vec == None {
//                 return None;
//             }
//             let init_vec = init_vec.unwrap();
//             // serialize both "initialization vector id"(unencrypted) and "T"(encrypted)(named "bytea" here)
//             let key = aes::AesKey::new_decrypt(AES_KEY).unwrap();
//             let mut output = vec![0_u8; x.len() - 4];
//             aes::aes_ige(
//                 &x[4..x.len()],
//                 &mut output,
//                 &key,
//                 &mut init_vec.clone(),
//                 Mode::Decrypt,
//             );
//             let offset: u32 = bincode::deserialize(&(output[0..4])).unwrap();
//             match bincode::deserialize(&output[4..(offset as usize + 4)]) {
//                 Ok(x) => Some(x),
//                 Err(_) => None,
//             }
//         }
//         Err(_) => None,
//     }
// }

// #[derive(Serialize, Deserialize, Debug)]
// pub struct TokenData{
//     // something that contain itself inside the token
// }

// pub struct Otoken<'a> {
//     id: i32,
//     data:Option<TokenData>,
//     runtime:Option<(&'a Cache,&'a DatabaseConnection)>
// }
// impl<'a> Otoken<'a> {
//     pub fn set_runtime(&mut self,conn:&'a DatabaseConnection,cache:&'a Cache){
//         self.runtime=Some((cache,conn));
//     }
//     pub fn set_data(&mut self,data:TokenData){
//         self.data=Some(data);
//     }
//     pub async fn encrypt(&self) -> String
//     {
//         assert!(self.runtime.is_some());
//         encode(self.data.as_ref().unwrap(), self.runtime.unwrap().0, self.runtime.unwrap().1).await
//     }
//     // pub async fn link(){}
//     // pub async fn decrypt(inp: &str, conn: &DatabaseConnection, cache: &mut Cache) ->Option<Otoken>
//     // {
//     //     let result:Option<Otoken>=decode(inp, cache, conn).await;
//     //     todo!()
//     // }
//     // pub async fn revoke() {
//     //     todo!();
//     // }
// }

// #[cfg(test)]
// mod test {
//     use actix_web::rt::time::sleep;
//     use sea_orm::{ConnectionTrait, Database, DbBackend, Schema};

//     use super::*;
//     use futures::future::join_all;
//     use std::sync::Arc;
//     use std::time;

//     #[actix_web::test]
//     async fn crypto_test() {
//         const SCALE:usize=300;
//         impl Clone for AppState {
//             fn clone(&self) -> Self {
//                 AppState {
//                     conn: self.conn.clone(),
//                     cache: self.cache.clone(),
//                 }
//             }
//         }
//         // let db: DatabaseConnection = MockDatabase::new(DatabaseBackend::Postgres).into_connection();
//         let db = Database::connect("sqlite::memory:").await.unwrap();
//         let schema = Schema::new(DbBackend::Sqlite);

//         let stmt = schema.create_table_from_entity(Token);
//         db.execute(db.get_database_backend().build(&stmt))
//             .await
//             .unwrap();

//         struct AppState {
//             conn: Arc<DatabaseConnection>,
//             cache: Arc<Cache>,
//         }
//         let state = AppState {
//             conn: Arc::new(db),
//             cache: Arc::new(Cache::new(50)),
//         };

//         #[derive(Serialize, Deserialize,Clone,PartialEq)]
//         struct Auth{
//             issuer_id:usize,
//         }

//         async fn spawn_one_thread(state: AppState) -> Option<bool> {
//             let mut rng = rand::thread_rng();
//             let random: usize = rng.gen();
//             let payload=Auth{issuer_id:random};
//             let token=encode(payload.clone(),&(*state.cache),&(*state.conn)).await;
//             sleep(time::Duration::from_millis(10)).await;

//             let decrypto:Option<Auth>=decode(&token, &(*state.cache),&(*state.conn)).await;

//             match decrypto {
//                 Some(x) => Some(payload==x),
//                 None => None,
//             }
//         }

//         let mut promises = Vec::new();

//         for _ in 0..SCALE {
//             promises.push(spawn_one_thread(state.clone()));
//         }

//         let result = join_all(promises).await;

//         // dbg!(&result);
//         assert_eq!(result,vec![Some(true);SCALE]);
//     }

//     #[actix_web::test]
//     async fn cache_test() {
//         const SCALE:usize=3000;
//         impl Clone for AppState {
//             fn clone(&self) -> Self {
//                 AppState {
//                     conn: self.conn.clone(),
//                     cache: self.cache.clone(),
//                 }
//             }
//         }
//         // let db: DatabaseConnection = MockDatabase::new(DatabaseBackend::Postgres).into_connection();
//         let db = Database::connect("sqlite::memory:").await.unwrap();
//         let schema = Schema::new(DbBackend::Sqlite);

//         let stmt = schema.create_table_from_entity(Token);
//         db.execute(db.get_database_backend().build(&stmt))
//             .await
//             .unwrap();

//         struct AppState {
//             conn: Arc<DatabaseConnection>,
//             cache: Arc<Cache>,
//         }
//         let state = AppState {
//             conn: Arc::new(db),
//             cache: Arc::new(Cache::new(100)),
//         };

//         async fn spawn_one_thread(state: AppState) -> Option<bool> {
//             let mut rng = rand::thread_rng();
//             let random: InitVecType = rng.gen();

//             let key = state.cache.insert(random, &(*state.conn)).await;
//             sleep(time::Duration::from_millis(10)).await;
//             let random_output = state.cache.retrieve(key, &(*state.conn)).await;
//             match random_output {
//                 Some(x) => Some(x == random),
//                 None => None,
//             }
//         }

//         let mut promises = Vec::new();

//         for _ in 0..SCALE {
//             promises.push(spawn_one_thread(state.clone()));
//         }

//         let result = join_all(promises).await;

//         // dbg!(&result);
//         assert_eq!(result,vec![Some(true);SCALE]);
//     }
// }
