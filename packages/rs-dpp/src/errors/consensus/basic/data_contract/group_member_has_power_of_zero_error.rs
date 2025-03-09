use crate::errors::consensus::basic::BasicError;
use crate::errors::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Member {member_id} has a power of 0, which is not allowed")]
#[platform_serialize(unversioned)]
#[cfg_attr(feature = "apple", ferment_macro::export)]
pub struct GroupMemberHasPowerOfZeroError {
    pub member_id: Identifier,
}

impl GroupMemberHasPowerOfZeroError {
    pub fn new(member_id: Identifier) -> Self {
        Self { member_id }
    }

    pub fn member_id(&self) -> Identifier {
        self.member_id
    }
}

impl From<GroupMemberHasPowerOfZeroError> for ConsensusError {
    fn from(err: GroupMemberHasPowerOfZeroError) -> Self {
        Self::BasicError(BasicError::GroupMemberHasPowerOfZeroError(err))
    }
}
