use crate::consensus::signature::signature_error::SignatureError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use thiserror::Error;

use bincode::{Decode, DecodeUntrusted, Encode};

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
#[error("RedPallas signing error {message}")]
#[platform_serialize(unversioned)]
pub struct BasicRedPallasError {
    message: String,
}

/*

DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

*/

impl BasicRedPallasError {
    pub fn new(message: String) -> Self {
        Self { message }
    }
}

impl From<BasicRedPallasError> for ConsensusError {
    fn from(err: BasicRedPallasError) -> Self {
        Self::SignatureError(SignatureError::BasicRedPallasError(err))
    }
}
