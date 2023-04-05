use crate::consensus::signature::signature_error::SignatureError;
use crate::consensus::ConsensusError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[error("Invalid State Transition signature")]
pub struct InvalidStateTransitionSignatureError;

/*

DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

*/

impl InvalidStateTransitionSignatureError {
    pub fn new() -> Self {
        Self::default()
    }
}

impl From<InvalidStateTransitionSignatureError> for ConsensusError {
    fn from(err: InvalidStateTransitionSignatureError) -> Self {
        Self::SignatureError(SignatureError::InvalidStateTransitionSignatureError(err))
    }
}
