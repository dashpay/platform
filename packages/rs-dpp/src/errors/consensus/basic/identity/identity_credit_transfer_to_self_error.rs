use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
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
    PlatformDeserialize,
    Default,
)]
#[error("Credits transfer recipient must be another identity")]
#[platform_serialize(unversioned)]
pub struct IdentityCreditTransferToSelfError {}

impl IdentityCreditTransferToSelfError {}

impl From<IdentityCreditTransferToSelfError> for ConsensusError {
    fn from(err: IdentityCreditTransferToSelfError) -> Self {
        Self::BasicError(BasicError::IdentityCreditTransferToSelfError(err))
    }
}
