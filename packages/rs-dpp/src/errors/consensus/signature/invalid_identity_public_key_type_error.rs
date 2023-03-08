use thiserror::Error;

use crate::consensus::signature::SignatureError;
use crate::consensus::ConsensusError;
use crate::identity::KeyType;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Unsupported signature type {public_key_type}. Please use ECDSA (0), BLS (1) or ECDSA_HASH160 (2) keys to sign the state transition")]
pub struct InvalidIdentityPublicKeyTypeError {
    public_key_type: KeyType,
}

impl InvalidIdentityPublicKeyTypeError {
    pub fn new(public_key_type: KeyType) -> Self {
        Self { public_key_type }
    }

    pub fn public_key_type(&self) -> KeyType {
        self.public_key_type.clone()
    }
}

impl From<InvalidIdentityPublicKeyTypeError> for ConsensusError {
    fn from(err: InvalidIdentityPublicKeyTypeError) -> Self {
        Self::SignatureError(SignatureError::InvalidIdentityPublicKeyTypeError(err))
    }
}
