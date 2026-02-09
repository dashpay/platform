use crate::consensus::signature::signature_error::SignatureError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Uncompressed public keys are not allowed: expected 33 bytes (compressed), got {public_key_size} bytes")]
#[platform_serialize(unversioned)]
pub struct UncompressedPublicKeyNotAllowedError {
    public_key_size: usize,
}

/*

DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

*/

impl UncompressedPublicKeyNotAllowedError {
    pub fn new(public_key_size: usize) -> Self {
        Self { public_key_size }
    }

    pub fn public_key_size(&self) -> usize {
        self.public_key_size
    }
}

impl From<UncompressedPublicKeyNotAllowedError> for ConsensusError {
    fn from(err: UncompressedPublicKeyNotAllowedError) -> Self {
        Self::SignatureError(SignatureError::UncompressedPublicKeyNotAllowedError(err))
    }
}
