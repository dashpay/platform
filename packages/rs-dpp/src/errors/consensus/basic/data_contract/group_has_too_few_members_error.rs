use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::data_contract::GroupContractPosition;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use std::error::Error;
use std::fmt;

/// Error indicating that a group contains too few members to be valid.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize)]
#[platform_serialize(unversioned)]
pub struct GroupHasTooFewMembersError {
    group_id: Option<GroupContractPosition>,
}

impl GroupHasTooFewMembersError {
    /// Creates a new error for the given group ID.
    pub fn new(group_id: Option<GroupContractPosition>) -> Self {
        Self { group_id }
    }

    /// Returns the ID of the group that has too few members.
    pub fn group_id(&self) -> Option<GroupContractPosition> {
        self.group_id
    }
}

impl fmt::Display for GroupHasTooFewMembersError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.group_id {
            Some(id) => write!(f, "Group {} must contain at least two members", id),
            None => write!(f, "Group must contain at least two members"),
        }
    }
}

impl Error for GroupHasTooFewMembersError {}

impl From<GroupHasTooFewMembersError> for ConsensusError {
    fn from(err: GroupHasTooFewMembersError) -> Self {
        Self::BasicError(BasicError::GroupHasTooFewMembersError(err))
    }
}
