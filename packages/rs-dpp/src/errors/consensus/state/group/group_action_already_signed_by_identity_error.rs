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
"Group action with action ID {} has already been signed by identity {} for data contract {} at group position {}",
    action_id,
    identity_id,
    data_contract_id,
    group_contract_position
)]
#[platform_serialize(unversioned)]
pub struct GroupActionAlreadySignedByIdentityError {
    identity_id: Identifier,
    data_contract_id: Identifier,
    group_contract_position: GroupContractPosition,
    action_id: Identifier,
}

impl GroupActionAlreadySignedByIdentityError {
    pub fn new(
        identity_id: Identifier,
        data_contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        action_id: Identifier,
    ) -> Self {
        Self {
            identity_id,
            data_contract_id,
            group_contract_position,
            action_id,
        }
    }

    pub fn identity_id(&self) -> Identifier {
        self.identity_id
    }

    pub fn data_contract_id(&self) -> Identifier {
        self.data_contract_id
    }

    pub fn group_contract_position(&self) -> GroupContractPosition {
        self.group_contract_position
    }

    pub fn action_id(&self) -> Identifier {
        self.action_id
    }
}

impl From<GroupActionAlreadySignedByIdentityError> for ConsensusError {
    fn from(err: GroupActionAlreadySignedByIdentityError) -> Self {
        Self::StateError(StateError::GroupActionAlreadySignedByIdentityError(err))
    }
}
