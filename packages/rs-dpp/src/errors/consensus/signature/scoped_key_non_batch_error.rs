use crate::consensus::signature::SignatureError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Scoped key {public_key_id} cannot sign a non-batch transition")]
#[platform_serialize(unversioned)]
pub struct ScopedKeyNonBatchError {
    public_key_id: u32,
}
impl ScopedKeyNonBatchError {
    pub fn new(public_key_id: u32) -> Self {
        Self { public_key_id }
    }
    pub fn public_key_id(&self) -> &u32 {
        &self.public_key_id
    }
}
impl From<ScopedKeyNonBatchError> for ConsensusError {
    fn from(error: ScopedKeyNonBatchError) -> Self {
        Self::SignatureError(SignatureError::ScopedKeyNonBatchError(error))
    }
}
