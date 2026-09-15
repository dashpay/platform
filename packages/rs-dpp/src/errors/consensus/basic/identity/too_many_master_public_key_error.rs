use crate::consensus::basic::BasicError;
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
#[error(
    "Identity is trying to be created with more than one master key. Please only use one master key."
)]
#[platform_serialize(unversioned)]
pub struct TooManyMasterPublicKeyError;

/*

DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

*/

impl TooManyMasterPublicKeyError {
    pub fn new() -> Self {
        Self
    }
}
impl From<TooManyMasterPublicKeyError> for ConsensusError {
    fn from(err: TooManyMasterPublicKeyError) -> Self {
        Self::BasicError(BasicError::TooManyMasterPublicKeyError(err))
    }
}
