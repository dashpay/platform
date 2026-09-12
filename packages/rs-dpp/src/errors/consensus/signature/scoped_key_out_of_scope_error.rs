use crate::consensus::signature::SignatureError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Batch member is outside key {public_key_id} scope")]
#[platform_serialize(unversioned)]
pub struct ScopedKeyOutOfScopeError {
    public_key_id: u32,
}
impl ScopedKeyOutOfScopeError {
    pub fn new(public_key_id: u32) -> Self {
        Self { public_key_id }
    }
    pub fn public_key_id(&self) -> &u32 {
        &self.public_key_id
    }
}
impl From<ScopedKeyOutOfScopeError> for ConsensusError {
    fn from(error: ScopedKeyOutOfScopeError) -> Self {
        Self::SignatureError(SignatureError::ScopedKeyOutOfScopeError(error))
    }
}
