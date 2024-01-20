use hickory_resolver::proto::error;
use k256::ecdsa::{
    signature::{Signer, Verifier},
    Signature, SigningKey, VerifyingKey,
};
use rand::rngs::OsRng;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tracing::Span;

use crate::init::config::GlobalConfig;
use blake2::{Blake2b512, Digest};

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("bincode: `{0}`")]
    Bincode(#[from] bincode::Error),
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

#[derive(Serialize, Deserialize)]
struct Signed {
    data: Vec<u8>,
    signature: Signature,
}
pub struct CryptoController {
    salt: Vec<u8>,
    signing_key: SigningKey,
    verifying_key: VerifyingKey,
}

impl CryptoController {
    #[tracing::instrument(parent=span,name="crypto_construct",level = "info",skip_all)]
    pub fn new(config: &GlobalConfig, span: &Span) -> Self {
        let salt = config.database.salt.as_bytes().to_vec();

        let signing_key = SigningKey::random(&mut OsRng);

        let verifying_key = *signing_key.verifying_key();

        Self {
            salt,
            signing_key,
            verifying_key,
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
    /// serialize and sign the object with blake2b512, append the signature and return
    #[tracing::instrument(level = "debug", skip_all)]
    pub fn encode<M: Serialize>(&self, obj: M) -> Result<String> {
        let raw = bincode::serialize(&obj)?;

        let signature: Signature = self.signing_key.sign(&raw);

        let signed = Signed {
            data: raw,
            signature,
        };
        Ok(base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD_NO_PAD,
            bincode::serialize(&signed)?,
        ))
    }
    /// extract signature and object of encoded bytes(serde will handle it)
    ///
    /// check signature and return the object
    ///
    /// Error if signature invaild
    #[tracing::instrument(level = "debug", skip_all)]
    pub fn decode<M: DeserializeOwned>(&self, raw: String) -> Result<M> {
        let raw = base64::Engine::decode(&base64::engine::general_purpose::STANDARD_NO_PAD, raw)?;
        let raw: Signed = bincode::deserialize(&raw)?;
        let signature = raw.signature;

        self.verifying_key
            .verify(&raw.data, &signature)
            .map_err(|_| Error::InvalidSignature)?;

        let obj = bincode::deserialize(&raw.data)?;
        Ok(obj)
    }
}
