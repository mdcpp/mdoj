use ring::{
    digest, rand,
    signature::{self, KeyPair},
};
use serde::{de::DeserializeOwned, Serialize};

use crate::{init::config::GlobalConfig, report_internal};

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("`{0}`")]
    Bincode(#[from] bincode::Error),
    #[error("Invalid signature")]
    InvalidSignature,
}

impl From<Error> for tonic::Status {
    fn from(value: Error) -> Self {
        match value {
            Error::Bincode(_) => report_internal!(debug, "`{}`", value),
            Error::InvalidSignature => report_internal!(trace, "`{}`", value),
        }
    }
}

pub struct CryptoController {
    signer: signature::Ed25519KeyPair,
    salt: Vec<u8>,
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
    #[tracing::instrument(level = "info")]
    pub fn new(config: &GlobalConfig) -> Self {
        let rng = rand::SystemRandom::new();
        let pkcs8_bytes = signature::Ed25519KeyPair::generate_pkcs8(&rng).unwrap();
        let signer = signature::Ed25519KeyPair::from_pkcs8(pkcs8_bytes.as_ref()).unwrap();

        let salt = config.database.salt.as_bytes().to_vec();
        Self { signer, salt }
    }
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
    pub fn hash(&self, src: &str) -> HashValue {
        let hashed = digest::digest(
            &digest::SHA256,
            &[src.as_bytes(), self.salt.as_slice()].concat(),
        )
        .as_ref()
        .to_vec();
        HashValue(hashed)
    }
    #[tracing::instrument(level = "trace", skip_all)]
    pub fn sign(&self, src: &str) -> Vec<u8> {
        self.signer.sign(src.as_bytes()).as_ref().to_vec()
    }
    #[tracing::instrument(level = "trace", skip_all)]
    pub fn verify(&self, src: &[u8], signature: &[u8]) -> bool {
        let peer_public_key = signature::UnparsedPublicKey::new(
            &signature::ED25519,
            self.signer.public_key().as_ref(),
        );
        peer_public_key.verify(src, signature).is_ok()
    }
    #[tracing::instrument(level = "trace", skip_all)]
    pub fn encode<M: Serialize>(&self, obj: M) -> Result<Vec<u8>> {
        let mut raw = bincode::serialize(&obj)?;
        let signature = self.signer.sign(&raw);

        raw.extend(signature.as_ref());

        Ok(raw)
    }
    #[tracing::instrument(level = "trace", skip_all)]
    pub fn decode<M: DeserializeOwned>(&self, raw: &[u8]) -> Result<M> {
        let (raw, signature) = raw.split_at(raw.len() - 64);
        if !self.verify(raw, signature) {
            return Err(Error::InvalidSignature);
        }
        Ok(bincode::deserialize(raw)?)
    }
}

// #[cfg(feature = "unsecured-log")]
