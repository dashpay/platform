use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

use crate::block::epoch::EpochIndex;
use bincode::{Decode, Encode};

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error(
    "Contested documents are not allowed until epoch {target_epoch}. Current epoch is {current_epoch}"
)]
#[platform_serialize(unversioned)]
pub struct ContestedDocumentsTemporarilyNotAllowedError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    current_epoch: EpochIndex,
    target_epoch: EpochIndex,
}

impl ContestedDocumentsTemporarilyNotAllowedError {
    pub fn new(current_epoch: EpochIndex, target_epoch: EpochIndex) -> Self {
        Self {
            current_epoch,
            target_epoch,
        }
    }
}

impl From<ContestedDocumentsTemporarilyNotAllowedError> for ConsensusError {
    fn from(err: ContestedDocumentsTemporarilyNotAllowedError) -> Self {
        Self::BasicError(BasicError::ContestedDocumentsTemporarilyNotAllowedError(
            err,
        ))
    }
}
