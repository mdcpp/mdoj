use ring::digest;

use crate::init::config::CONFIG;

pub fn hash(src: &str) -> Vec<u8> {
    let config = CONFIG.get().unwrap();
    digest::digest(
        &digest::SHA256,
        &[src.as_bytes(), config.database.salt.as_bytes()].concat(),
    )
    .as_ref()
    .to_vec()
}

pub fn hash_eq(src: &str, tar: &Vec<u8>) -> bool {
    let hashed = hash(src);
    let mut result = true;
    for (a, b) in hashed.iter().zip(tar.iter()) {
        if *a != *b {
            result = false;
        }
    }
    result
}
