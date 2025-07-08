use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Group action is not allowed during transition: {}", transition_type)]
#[platform_serialize(unversioned)]
pub struct GroupActionNotAllowedOnTransitionError {
    transition_type: String,
}

impl GroupActionNotAllowedOnTransitionError {
    pub fn new(transition_type: String) -> Self {
        Self { transition_type }
    }

    pub fn transition_type(&self) -> &str {
        &self.transition_type
    }
}

impl From<GroupActionNotAllowedOnTransitionError> for ConsensusError {
    fn from(err: GroupActionNotAllowedOnTransitionError) -> Self {
        Self::BasicError(BasicError::GroupActionNotAllowedOnTransitionError(err))
    }
}
