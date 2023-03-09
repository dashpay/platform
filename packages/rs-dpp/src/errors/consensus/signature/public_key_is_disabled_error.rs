use thiserror::Error;

use crate::consensus::signature::SignatureError;
use crate::consensus::ConsensusError;
use crate::identity::KeyID;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Identity key {public_key_id} is disabled")]
pub struct PublicKeyIsDisabledError {
    public_key_id: KeyID,
}

impl PublicKeyIsDisabledError {
    pub fn new(public_key_id: KeyID) -> Self {
        Self { public_key_id }
    }

    pub fn public_key_id(&self) -> KeyID {
        self.public_key_id.clone()
    }
}

impl From<PublicKeyIsDisabledError> for ConsensusError {
    fn from(err: PublicKeyIsDisabledError) -> Self {
        Self::SignatureError(SignatureError::PublicKeyIsDisabledError(err))
    }
}
