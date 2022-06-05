use std::sync::{Mutex, Arc};

use crate::entity::{token_table as token, prelude::TokenTable as Token};
use openssl::symm::Mode;
use openssl::{aes,base64};
use serde::{Deserialize, Serialize};
use rand::prelude::*;
use lru::LruCache;


const AES_KEY: &[u8; 32] = include_bytes!["../../config/aes"];


type salt_type=[u8; 32];
pub struct Cache{
    lru: LruCache<usize, salt_type>,
    state: usize
}

impl Cache{
    pub fn new()->Self{
        Cache{
           lru:LruCache::new(100),
           state: 0
        }
    }
}




pub fn encode<T>(payload:T,salt:salt_type,cache:&mut Cache)->String where T:Serialize{
    // regist salt
    let id=cache.state;
    cache.lru.put(id, salt); 
    cache.state=cache.state+1;
    // serialize payload
    let bytea = bincode::serialize(&payload).unwrap();
    let len=bincode::serialize(&(bytea.len() as u32)).unwrap();
    let id=bincode::serialize(&(id as u32)).unwrap();
    // fill bytea with random bytes (AES IGE require the input bytea to be multiple of 16)
    let mut rng = rand::thread_rng();
    let mut seed=rng.gen::<[u8;32]>().to_vec();
    seed.truncate(16-bytea.len()%16+12);
    let bytea=[len,bytea,seed.to_owned()].concat();
    // encrypt aes256
    let key=aes::AesKey::new_encrypt(AES_KEY).unwrap();
    let mut output = vec![0u8; bytea.len()];
    aes::aes_ige(&bytea,&mut output,&key,& mut salt.clone(),Mode::Encrypt);
    // encrypt base64
    base64::encode_block(&[id,output].concat())
}

fn decode<'a, T>(input:&str,salt:salt_type,cache:Cache)->Option<T> where T:Deserialize<'a>{

    match base64::decode_block(&input){
        Ok(x) => {
            let id:u32=bincode::deserialize_from(&x[0..4]).unwrap();
            // cache.lru.get<u32>(&id);
            // get salt by id
            let key=aes::AesKey::new_decrypt(AES_KEY).unwrap();
            let mut output=vec![0_u8;x.len()-4];
            aes::aes_ige(&x[4..(x.len()-1)], &mut output, &key, &mut salt.clone(), Mode::Decrypt);
            let offset:u32=bincode::deserialize(&(output[0..4])).unwrap() ;
            let output: &'a Vec<u8>=& output.clone();
            match bincode::deserialize::<'a>(& output[4..(offset as usize +4)]) {
                Ok(x) =>Some(x),
                Err(_) => None,
            }
        },
        Err(_) => None,
    }

}

#[cfg(test)]
mod test{
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