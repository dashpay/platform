use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::data_contract::GroupContractPosition;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error(
    "Identity {} is not a member of the group for data contract {} at position {}",
    identity_id,
    data_contract_id,
    group_contract_position
)]
#[platform_serialize(unversioned)]
pub struct IdentityNotMemberOfGroupError {
    identity_id: Identifier,
    data_contract_id: Identifier,
    group_contract_position: GroupContractPosition,
}

impl IdentityNotMemberOfGroupError {
    pub fn new(
        identity_id: Identifier,
        data_contract_id: Identifier,
        group_contract_position: GroupContractPosition,
    ) -> Self {
        Self {
            identity_id,
            data_contract_id,
            group_contract_position,
        }
    }

    pub fn identity_id(&self) -> &Identifier {
        &self.identity_id
    }

    pub fn data_contract_id(&self) -> &Identifier {
        &self.data_contract_id
    }

    pub fn group_contract_position(&self) -> GroupContractPosition {
        self.group_contract_position
    }
}

impl From<IdentityNotMemberOfGroupError> for ConsensusError {
    fn from(err: IdentityNotMemberOfGroupError) -> Self {
        Self::StateError(StateError::IdentityNotMemberOfGroupError(err))
    }
}
