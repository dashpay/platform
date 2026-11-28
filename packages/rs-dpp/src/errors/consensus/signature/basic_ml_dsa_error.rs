use crate::consensus::signature::signature_error::SignatureError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use thiserror::Error;

use bincode::{Decode, DecodeUntrusted, Encode};

/// An ML_DSA_65 key's proof of possession does not verify: the key or the signature has the
/// wrong size, the signature is not well formed, or it is not the key's signature over the
/// transition.
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
    PlatformDeserializeTrusted,
    PlatformDeserializeUntrusted,
    DecodeUntrusted,
)]
#[error("ml-dsa signing error {message}")]
#[platform_serialize(unversioned)]
pub struct BasicMLDSAError {
    message: String,
}

/*

DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

*/

impl BasicMLDSAError {
    pub fn new(message: String) -> Self {
        Self { message }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl From<BasicMLDSAError> for ConsensusError {
    fn from(err: BasicMLDSAError) -> Self {
        Self::SignatureError(SignatureError::BasicMLDSAError(err))
    }
}
