use crate::errors::consensus::state::state_error::StateError;
use crate::errors::consensus::ConsensusError;
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
    "Group action does not exist for data contract {} at group position {} with action ID {}",
    data_contract_id,
    group_contract_position,
    action_id
)]
#[platform_serialize(unversioned)]
pub struct GroupActionDoesNotExistError {
    data_contract_id: Identifier,
    group_contract_position: GroupContractPosition,
    action_id: Identifier,
}

impl GroupActionDoesNotExistError {
    pub fn new(
        data_contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        action_id: Identifier,
    ) -> Self {
        Self {
            data_contract_id,
            group_contract_position,
            action_id,
        }
    }

    pub fn data_contract_id(&self) -> Identifier {
        self.data_contract_id
    }

    pub fn group_contract_position(&self) -> &GroupContractPosition {
        &self.group_contract_position
    }

    pub fn action_id(&self) -> Identifier {
        self.action_id
    }
}

impl From<GroupActionDoesNotExistError> for ConsensusError {
    fn from(err: GroupActionDoesNotExistError) -> Self {
        Self::StateError(StateError::GroupActionDoesNotExistError(err))
    }
}
