use thiserror::Error;

use crate::consensus::ConsensusError;
use crate::identity::KeyType;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Invalid signature type")]
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
        Self::InvalidIdentityPublicKeyTypeError(err)
    }
}
