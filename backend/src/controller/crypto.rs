use rand::SeedableRng;
use rand_hc::Hc128Rng;
use serde::{de::DeserializeOwned, Serialize};
use spin::Mutex;
use tracing::Span;
use k256::{SecretKey, Secp256k1, PublicKey};

use crate::{init::config::GlobalConfig, report_internal};
use blake2::{Blake2b512, Digest};

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("`{0}`")]
    Bincode(#[from] bincode::Error),
    #[error("Invalid signature")]
    InvalidSignature,
    #[error("Encode error")]
    Encode,
    #[error("Decode error")]
    Decode,
}

impl From<Error> for tonic::Status {
    fn from(value: Error) -> Self {
        match value {
            Error::Bincode(_) => report_internal!(debug, "`{}`", value),
            Error::InvalidSignature => report_internal!(trace, "`{}`", value),
            Error::Encode => report_internal!(trace, "`{}`", value),
            Error::Decode => tonic::Status::invalid_argument("signature is invalid"),
        }
    }
}

pub struct CryptoController {
    salt: Vec<u8>,
    rng: Mutex<Hc128Rng>,
    secret:SecretKey,
    public:PublicKey,
}

#[derive(PartialEq, Eq)]
pub struct HashValue(Vec<u8>);

impl From<Vec<u8>> for HashValue {
    fn from(v: Vec<u8>) -> Self {
        Self(v)
    }
}

impl From<HashValue> for Vec<u8> {
    fn from(v: HashValue) -> Self {
        v.0
    }
}

impl CryptoController {
    #[tracing::instrument(parent=span,name="crypto_construct",level = "info",skip_all)]
    pub fn new(config: &GlobalConfig, span: &Span) -> Self {
        let salt = config.database.salt.as_bytes().to_vec();

        let mut rng = Hc128Rng::from_entropy();
        let secret=SecretKey::random(&mut rng);
        let public=secret.public_key();
        Self {
            salt,
            rng: Mutex::new(rng),
            secret,public
        }
    }
    #[tracing::instrument(name = "crypto_hasheq_controller", level = "debug", skip_all)]
    pub fn hash_eq(&self, src: &str, tar: &[u8]) -> bool {
        let hashed: Vec<u8> = self.hash(src).into();
        let mut result = true;
        for (a, b) in hashed.iter().zip(tar.iter()) {
            if *a != *b {
                result = false;
            }
        }
        result
    }
    #[tracing::instrument(name = "crypto_hash_controller", level = "debug", skip_all)]
    pub fn hash(&self, src: &str) -> HashValue {
        let mut hasher = Blake2b512::new();
        hasher.update(&[src.as_bytes(), self.salt.as_slice()].concat());

        let hashed = hasher.finalize();
        HashValue(hashed.to_vec())
    }
    #[tracing::instrument(level = "trace", skip_all)]
    pub fn encode<M: Serialize>(&self, obj: M) -> Result<Vec<u8>> {
        let mut raw = bincode::serialize(&obj)?;

        todo!()
    }
    #[tracing::instrument(level = "trace", skip_all)]
    pub fn decode<M: DeserializeOwned>(&self, raw: Vec<u8>) -> Result<M> {
        todo!()
    }
}

// #[cfg(feature = "unsecured-log")]
