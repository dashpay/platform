use thiserror::Error;

use crate::consensus::ConsensusError;
use crate::identity::SecurityLevel;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Invalid signature public key")]
pub struct InvalidSignaturePublicKeyError {
    public_key: Vec<u8>,
}

impl InvalidSignaturePublicKeyError {
    pub fn new(public_key: Vec<u8>) -> Self {
        Self { public_key }
    }

    pub fn public_key(&self) -> Vec<u8> {
        self.public_key.clone()
    }
}

impl From<InvalidSignaturePublicKeyError> for ConsensusError {
    fn from(err: InvalidSignaturePublicKeyError) -> Self {
        Self::InvalidSignaturePublicKeyError(err)
    }
}
