use crate::consensus::signature::SignatureError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use thiserror::Error;

#[derive(
    Error,
    Debug,
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserializeTrusted,
    PlatformDeserializeUntrusted,
    DecodeUntrusted,
)]
#[error("Batch member is outside the contract bounds of key {public_key_id}")]
#[platform_serialize(unversioned)]
pub struct ContractBoundedKeyOutOfBoundsError {
    public_key_id: u32,
}
impl ContractBoundedKeyOutOfBoundsError {
    pub fn new(public_key_id: u32) -> Self {
        Self { public_key_id }
    }
    pub fn public_key_id(&self) -> &u32 {
        &self.public_key_id
    }
}
impl From<ContractBoundedKeyOutOfBoundsError> for ConsensusError {
    fn from(error: ContractBoundedKeyOutOfBoundsError) -> Self {
        Self::SignatureError(SignatureError::ContractBoundedKeyOutOfBoundsError(error))
    }
}
