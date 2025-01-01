use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::prelude::Identifier;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;
#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error(
    "Invalid action id {}, expected {}",
    invalid_action_id,
    expected_action_id
)]
#[platform_serialize(unversioned)]
pub struct InvalidActionIdError {
    expected_action_id: Identifier,
    invalid_action_id: Identifier,
}

impl InvalidActionIdError {
    pub fn new(expected_action_id: Identifier, invalid_action_id: Identifier) -> Self {
        Self {
            expected_action_id,
            invalid_action_id,
        }
    }

    pub fn expected_action_id(&self) -> Identifier {
        self.expected_action_id
    }

    pub fn invalid_action_id(&self) -> Identifier {
        self.invalid_action_id
    }
}

impl From<InvalidActionIdError> for ConsensusError {
    fn from(err: InvalidActionIdError) -> Self {
        Self::BasicError(BasicError::InvalidActionIdError(err))
    }
}
