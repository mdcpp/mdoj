// use k256::ecdsa::{SigningKey, VerifyingKey};
use rand::{rngs::OsRng, Rng};
use serde::{de::DeserializeOwned, Serialize};

use crate::config::CONFIG;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use blake2::{Blake2b512, Digest};
use tracing::instrument;

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("postcard: `{0}`")]
    Bincode(#[from] postcard::Error),
    #[error("Invalid signature")]
    InvalidSignature,
    #[error("base64: `{0}`")]
    Base64(#[from] base64::DecodeError),
}

impl From<Error> for tonic::Status {
    fn from(value: Error) -> Self {
        tracing::trace!(reason = ?value, "crypto_error");
        tonic::Status::invalid_argument("Invalid signature")
    }
}

pub struct CryptoController {
    salt: Vec<u8>,
    xor_key: u8,
}

impl CryptoController {
    pub fn new() -> Self {
        let salt = CONFIG.database.salt.as_bytes().to_vec();
        Self {
            salt,
            xor_key: OsRng.gen(),
        }
    }
    /// hash `src` and compare hash value with `hashed`
    pub fn hash_eq(&self, src: &str, hashed: &[u8]) -> bool {
        let src_hashed: Vec<u8> = self.hash(src);
        let mut result = true;
        for (a, b) in src_hashed.iter().zip(hashed.iter()) {
            if *a != *b {
                result = false;
            }
        }
        result
    }
    /// get BLAKE2b-512 hashed bytes with salt
    pub fn hash(&self, src: &str) -> Vec<u8> {
        let mut hasher = Blake2b512::new();
        hasher.update([src.as_bytes(), self.salt.as_slice()].concat());

        let hashed = hasher.finalize();
        hashed.to_vec()
    }
    /// Serialize and calculate checksum and return
    ///
    /// Note that it shouldn't be an security measurement
    #[instrument(skip_all, level = "debug", ret(level = "debug"))]
    pub fn encode<M: Serialize>(&self, obj: M) -> Result<String> {
        let mut raw = postcard::to_allocvec(&obj)?;

        let checksum: u8 = raw
            .iter()
            .fold(self.xor_key ^ (raw.len() % 255) as u8, |acc, x| acc ^ x);
        raw.push(checksum);

        Ok(URL_SAFE_NO_PAD.encode(raw))
    }
    /// Extract checksum and object of encoded bytes(serde will handle it)
    ///
    /// check signature and return the object
    ///
    /// Error if signature invaild
    #[instrument(skip_all, level = "debug", err(level = "debug"))]
    pub fn decode<M: DeserializeOwned>(&self, raw: String) -> Result<M> {
        let mut raw = URL_SAFE_NO_PAD.decode(raw)?;

        let mut signature = raw.pop().ok_or(Error::InvalidSignature)?;

        signature ^= (raw.len() % 255) as u8;
        signature ^= raw.iter().fold(self.xor_key, |acc, x| acc ^ x);

        if signature != 0 {
            return Err(Error::InvalidSignature);
        }

        postcard::from_bytes(&raw).map_err(Into::into)
    }
}
