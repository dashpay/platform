use thiserror::Error;

use crate::identity::IdentityPublicKey;
use crate::ProtocolError;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Public key is disabled")]
pub struct PublicKeyIsDisabledError {
    public_key: IdentityPublicKey,
}

impl PublicKeyIsDisabledError {
    pub fn new(public_key: IdentityPublicKey) -> Self {
        Self { public_key }
    }

    pub fn public_key(&self) -> IdentityPublicKey {
        self.public_key.clone()
    }
}

impl From<PublicKeyIsDisabledError> for ProtocolError {
    fn from(err: PublicKeyIsDisabledError) -> Self {
        Self::PublicKeyIsDisabledError(err)
    }
}
