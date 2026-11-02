use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use thiserror::Error;

/// An approval of a team action the contract has no record of, active or closed: no member of
/// its seated team proposed it.
#[derive(
    Error,
    Debug,
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserializeTrusted,
    PlatformDeserializeUntrusted,
    DecodeUntrusted,
)]
#[error(
    "No team action {} was proposed on contract {}",
    action_id,
    contract_id
)]
#[platform_serialize(unversioned)]
pub struct ContractTeamActionDoesNotExistError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    contract_id: Identifier,
    action_id: Identifier,
}

impl ContractTeamActionDoesNotExistError {
    pub fn new(contract_id: Identifier, action_id: Identifier) -> Self {
        Self {
            contract_id,
            action_id,
        }
    }

    /// The moderated contract
    pub fn contract_id(&self) -> Identifier {
        self.contract_id
    }

    /// The team action the approval names
    pub fn action_id(&self) -> Identifier {
        self.action_id
    }
}

impl From<ContractTeamActionDoesNotExistError> for ConsensusError {
    fn from(err: ContractTeamActionDoesNotExistError) -> Self {
        Self::StateError(StateError::ContractTeamActionDoesNotExistError(err))
    }
}
