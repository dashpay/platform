use thiserror::Error;

use crate::consensus::signature::SignatureError;
use crate::consensus::ConsensusError;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Public key {public_key_id} doesn't exist")]
pub struct MissingPublicKeyError {
    public_key_id: u64,
}

impl MissingPublicKeyError {
    pub fn new(public_key_id: u64) -> Self {
        Self { public_key_id }
    }

    pub fn public_key_id(&self) -> u64 {
        self.public_key_id
    }
}

impl From<MissingPublicKeyError> for ConsensusError {
    fn from(err: MissingPublicKeyError) -> Self {
        Self::SignatureError(SignatureError::MissingPublicKeyError(err))
    }
}
