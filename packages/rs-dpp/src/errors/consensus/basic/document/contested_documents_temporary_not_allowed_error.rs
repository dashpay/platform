use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

use crate::prelude::Identifier;

use crate::block::epoch::EpochIndex;
use bincode::{Decode, Encode};

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error(
    "Contested documents are not allowed until epoch {target_epoch}. Current epoch is {current_epoch}"
)]
#[platform_serialize(unversioned)]
pub struct ContestedDocumentsTemporaryNotAllowedError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    current_epoch: EpochIndex,
    target_epoch: EpochIndex,
}

impl ContestedDocumentsTemporaryNotAllowedError {
    pub fn new(current_epoch: EpochIndex, target_epoch: EpochIndex) -> Self {
        Self {
            current_epoch,
            target_epoch,
        }
    }
}

impl From<ContestedDocumentsTemporaryNotAllowedError> for ConsensusError {
    fn from(err: ContestedDocumentsTemporaryNotAllowedError) -> Self {
        Self::BasicError(BasicError::ContestedDocumentsTemporaryNotAllowedError(err))
    }
}
