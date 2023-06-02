use crate::consensus::signature::signature_error::SignatureError;
use crate::consensus::ConsensusError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use bincode::{Decode, Encode};

#[derive(Error, Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Encode, Decode)]
#[error("signature should be empty {message}")]
pub struct SignatureShouldNotBePresentError {
    message: String,
}

/*

DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

*/

impl SignatureShouldNotBePresentError {
    pub fn new(message: String) -> Self {
        Self { message }
    }
}

impl From<SignatureShouldNotBePresentError> for ConsensusError {
    fn from(err: SignatureShouldNotBePresentError) -> Self {
        Self::SignatureError(SignatureError::SignatureShouldNotBePresentError(err))
    }
}
