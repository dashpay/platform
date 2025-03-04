use thiserror::Error;

use crate::identity::identity_public_key::IdentityPublicKey;
use crate::errors::ProtocolError;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Public key mismatched")]
#[cfg_attr(feature = "apple", ferment_macro::export)]
pub struct PublicKeyMismatchError {
    pub public_key: IdentityPublicKey,
}

impl PublicKeyMismatchError {
    pub fn new(public_key: IdentityPublicKey) -> Self {
        Self { public_key }
    }

    pub fn public_key(&self) -> IdentityPublicKey {
        self.public_key.clone()
    }
}

impl From<PublicKeyMismatchError> for ProtocolError {
    fn from(err: PublicKeyMismatchError) -> Self {
        Self::PublicKeyMismatchError(err)
    }
}
