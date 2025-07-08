use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("identity {member_id} was not found in group at position {group_position} in contract {contract_id}")]
#[platform_serialize(unversioned)]
pub struct IdentityMemberOfGroupNotFoundError {
    contract_id: Identifier,
    group_position: u16,
    member_id: Identifier,
}

impl IdentityMemberOfGroupNotFoundError {
    pub fn new(contract_id: Identifier, group_position: u16, member_id: Identifier) -> Self {
        Self {
            contract_id,
            group_position,
            member_id,
        }
    }

    pub fn contract_id(&self) -> &Identifier {
        &self.contract_id
    }

    pub fn group_position(&self) -> u16 {
        self.group_position
    }

    pub fn member_id(&self) -> &Identifier {
        &self.member_id
    }
}

impl From<IdentityMemberOfGroupNotFoundError> for ConsensusError {
    fn from(err: IdentityMemberOfGroupNotFoundError) -> Self {
        Self::StateError(StateError::IdentityMemberOfGroupNotFoundError(err))
    }
}
