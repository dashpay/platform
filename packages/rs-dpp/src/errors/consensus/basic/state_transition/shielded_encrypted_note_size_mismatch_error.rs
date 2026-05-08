use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error(
    "Shielded action encrypted_note has invalid size: expected {expected_size} bytes, got {actual_size} bytes"
)]
#[platform_serialize(unversioned)]
pub struct ShieldedEncryptedNoteSizeMismatchError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    expected_size: u32,
    actual_size: u32,
}

impl ShieldedEncryptedNoteSizeMismatchError {
    pub fn new(expected_size: u32, actual_size: u32) -> Self {
        Self {
            expected_size,
            actual_size,
        }
    }

    pub fn expected_size(&self) -> u32 {
        self.expected_size
    }

    pub fn actual_size(&self) -> u32 {
        self.actual_size
    }
}

impl From<ShieldedEncryptedNoteSizeMismatchError> for ConsensusError {
    fn from(err: ShieldedEncryptedNoteSizeMismatchError) -> Self {
        Self::BasicError(BasicError::ShieldedEncryptedNoteSizeMismatchError(err))
    }
}
