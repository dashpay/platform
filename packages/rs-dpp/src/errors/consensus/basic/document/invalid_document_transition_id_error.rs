use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use crate::prelude::Identifier;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error(
    "Invalid document transition id {}, expected {}",
    invalid_id,
    expected_id
)]
#[platform_serialize(unversioned)]
pub struct InvalidDocumentTransitionIdError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    expected_id: Identifier,
    invalid_id: Identifier,
}

impl InvalidDocumentTransitionIdError {
    pub fn new(expected_id: Identifier, invalid_id: Identifier) -> Self {
        Self {
            expected_id,
            invalid_id,
        }
    }

    pub fn expected_id(&self) -> Identifier {
        self.expected_id
    }

    pub fn invalid_id(&self) -> Identifier {
        self.invalid_id
    }
}

impl From<InvalidDocumentTransitionIdError> for ConsensusError {
    fn from(err: InvalidDocumentTransitionIdError) -> Self {
        Self::BasicError(BasicError::InvalidDocumentTransitionIdError(err))
    }
}
