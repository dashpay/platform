use thiserror::Error;

use crate::consensus::ConsensusError;
use crate::identity::IdentityPublicKey;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Public key mismatched")]
pub struct PublicKeyMismatchError {
    public_key: IdentityPublicKey,
}

impl PublicKeyMismatchError {
    pub fn new(public_key: IdentityPublicKey) -> Self {
        Self { public_key }
    }

    pub fn public_key(&self) -> IdentityPublicKey {
        self.public_key.clone()
    }
}

impl From<PublicKeyMismatchError> for ConsensusError {
    fn from(err: PublicKeyMismatchError) -> Self {
        Self::PublicKeyMismatchError(err)
    }
}
