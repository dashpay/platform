use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

use bincode::{Decode, Encode};

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
#[error("Instant lock proof signature is invalid or wasn't created recently. Pleases try chain asset lock proof instead.")]
#[platform_serialize(unversioned)]
pub struct InvalidInstantAssetLockProofSignatureError;

/*

DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

*/

impl InvalidInstantAssetLockProofSignatureError {
    pub fn new() -> Self {
        Self
    }
}
impl From<InvalidInstantAssetLockProofSignatureError> for ConsensusError {
    fn from(err: InvalidInstantAssetLockProofSignatureError) -> Self {
        Self::BasicError(BasicError::InvalidInstantAssetLockProofSignatureError(err))
    }
}
