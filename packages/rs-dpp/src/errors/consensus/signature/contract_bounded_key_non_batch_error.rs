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
#[error("Contract-bound authentication key {public_key_id} cannot sign a non-batch transition")]
#[platform_serialize(unversioned)]
pub struct ContractBoundedKeyNonBatchError {
    public_key_id: u32,
}
impl ContractBoundedKeyNonBatchError {
    pub fn new(public_key_id: u32) -> Self {
        Self { public_key_id }
    }
    pub fn public_key_id(&self) -> &u32 {
        &self.public_key_id
    }
}
impl From<ContractBoundedKeyNonBatchError> for ConsensusError {
    fn from(error: ContractBoundedKeyNonBatchError) -> Self {
        Self::SignatureError(SignatureError::ContractBoundedKeyNonBatchError(error))
    }
}
