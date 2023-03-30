use crate::consensus::basic::BasicError;
use thiserror::Error;
use crate::consensus::ConsensusError;

use crate::identity::KeyID;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Identity key {public_key_id} has invalid signature")]
pub struct InvalidIdentityKeySignatureError {
    public_key_id: KeyID,
}

impl InvalidIdentityKeySignatureError {
    pub fn new(public_key_id: KeyID) -> Self {
        Self { public_key_id }
    }

    pub fn public_key_id(&self) -> KeyID {
        self.public_key_id
    }
}

impl From<InvalidIdentityKeySignatureError> for ConsensusError {
    fn from(err: InvalidIdentityKeySignatureError) -> Self {
        Self::BasicError(BasicError::InvalidIdentityKeySignatureError(err))
    }
}
