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
"Group action has already been completed for data contract {} at group position {} with action ID {}",
    data_contract_id,
    group_contract_position,
    action_id
)]
#[platform_serialize(unversioned)]
pub struct GroupActionAlreadyCompletedError {
    data_contract_id: Identifier,
    group_contract_position: GroupContractPosition,
    action_id: Identifier,
}

impl GroupActionAlreadyCompletedError {
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

impl From<GroupActionAlreadyCompletedError> for ConsensusError {
    fn from(err: GroupActionAlreadyCompletedError) -> Self {
        Self::StateError(StateError::GroupActionAlreadyCompletedError(err))
    }
}
