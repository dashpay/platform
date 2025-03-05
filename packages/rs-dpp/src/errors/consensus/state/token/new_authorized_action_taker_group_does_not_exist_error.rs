use crate::data_contract::GroupContractPosition;
use crate::errors::consensus::state::state_error::StateError;
use crate::errors::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("The specified new authorized action taker group {group_contract_position} does not exist")]
#[platform_serialize(unversioned)]
#[cfg_attr(feature = "apple", ferment_macro::export)]
pub struct NewAuthorizedActionTakerGroupDoesNotExistError {
    pub group_contract_position: GroupContractPosition,
}

impl NewAuthorizedActionTakerGroupDoesNotExistError {
    pub fn new(group_contract_position: GroupContractPosition) -> Self {
        Self {
            group_contract_position,
        }
    }

    pub fn group_contract_position(&self) -> GroupContractPosition {
        self.group_contract_position
    }
}

impl From<NewAuthorizedActionTakerGroupDoesNotExistError> for ConsensusError {
    fn from(err: NewAuthorizedActionTakerGroupDoesNotExistError) -> Self {
        Self::StateError(StateError::NewAuthorizedActionTakerGroupDoesNotExistError(
            err,
        ))
    }
}
