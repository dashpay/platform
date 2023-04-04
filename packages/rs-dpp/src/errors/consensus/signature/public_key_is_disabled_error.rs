use thiserror::Error;

use crate::consensus::signature::signature_error::SignatureError;
use crate::consensus::ConsensusError;
use crate::identity::KeyID;
use serde::{Deserialize, Serialize};

#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[error("Identity key {public_key_id} is disabled")]
pub struct PublicKeyIsDisabledError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    public_key_id: KeyID,
}

impl PublicKeyIsDisabledError {
    pub fn new(public_key_id: KeyID) -> Self {
        Self { public_key_id }
    }

    pub fn public_key_id(&self) -> KeyID {
        self.public_key_id
    }
}

impl From<PublicKeyIsDisabledError> for ConsensusError {
    fn from(err: PublicKeyIsDisabledError) -> Self {
        Self::SignatureError(SignatureError::PublicKeyIsDisabledError(err))
    }
}
