use crate::consensus::signature::signature_error::SignatureError;
use crate::consensus::ConsensusError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use bincode::{Decode, Encode};

#[derive(Error, Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Encode, Decode)]
#[error("ecdsa signing error {message}")]
pub struct BasicECDSAError {
    message: String,
}

/*

DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

*/

impl BasicECDSAError {
    pub fn new(message: String) -> Self {
        Self { message }
    }
}

impl From<BasicECDSAError> for ConsensusError {
    fn from(err: BasicECDSAError) -> Self {
        Self::SignatureError(SignatureError::BasicECDSAError(err))
    }
}
