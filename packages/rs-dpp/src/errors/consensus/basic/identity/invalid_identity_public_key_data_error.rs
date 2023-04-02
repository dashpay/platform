use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::identity::KeyID;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// TODO missing setValidationError
#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[error("Invalid identity public key {public_key_id:?} data: {validation_error:?}")]
pub struct InvalidIdentityPublicKeyDataError {
    public_key_id: KeyID,
    validation_error: String,
}

impl InvalidIdentityPublicKeyDataError {
    pub fn new(public_key_id: KeyID, validation_error: String) -> Self {
        Self {
            public_key_id,
            validation_error,
        }
    }

    pub fn public_key_id(&self) -> KeyID {
        self.public_key_id
    }

    pub fn validation_error(&self) -> &str {
        &self.validation_error
    }
}
impl From<InvalidIdentityPublicKeyDataError> for ConsensusError {
    fn from(err: InvalidIdentityPublicKeyDataError) -> Self {
        Self::BasicError(BasicError::InvalidIdentityPublicKeyDataError(err))
    }
}
