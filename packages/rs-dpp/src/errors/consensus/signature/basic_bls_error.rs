use crate::consensus::signature::signature_error::SignatureError;
use crate::consensus::ConsensusError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[error("bls signing error {message}")]
pub struct BasicBLSError {
    message: String,
}

/*

DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

*/

impl BasicBLSError {
    pub fn new(message: String) -> Self {
        Self { message }
    }
}

impl From<BasicBLSError> for ConsensusError {
    fn from(err: BasicBLSError) -> Self {
        Self::SignatureError(SignatureError::BasicBLSError(err))
    }
}
