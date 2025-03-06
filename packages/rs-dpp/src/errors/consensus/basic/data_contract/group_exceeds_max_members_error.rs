use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Group exceeds the maximum allowed number of members: {max_members}")]
#[platform_serialize(unversioned)]
pub struct GroupExceedsMaxMembersError {
    max_members: u32,
}

impl GroupExceedsMaxMembersError {
    pub fn new(max_members: u32) -> Self {
        Self { max_members }
    }

    pub fn max_members(&self) -> u32 {
        self.max_members
    }
}

impl From<GroupExceedsMaxMembersError> for ConsensusError {
    fn from(err: GroupExceedsMaxMembersError) -> Self {
        Self::BasicError(BasicError::GroupExceedsMaxMembersError(err))
    }
}
