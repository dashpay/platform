use crate::consensus::signature::signature_error::SignatureError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error,
    Debug,
    Clone,
    PartialEq,
    Eq,
    Default,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserialize,
)]
#[error("Invalid State Transition signature")]
#[platform_serialize(unversioned)]
pub struct InvalidStateTransitionSignatureError {
    message: String,
}

/*

DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

*/

impl InvalidStateTransitionSignatureError {
    pub fn new(message: String) -> Self {
        Self { message }
    }

    pub fn message(&self) -> &String {
        &self.message
    }
}

impl From<InvalidStateTransitionSignatureError> for ConsensusError {
    fn from(err: InvalidStateTransitionSignatureError) -> Self {
        Self::SignatureError(SignatureError::InvalidStateTransitionSignatureError(err))
    }
}
