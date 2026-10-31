use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use thiserror::Error;

/// An approval of a team action that already ran: the approvals met what it needs and it is
/// closed.
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
#[error("Team action {} on contract {} already ran", action_id, contract_id)]
#[platform_serialize(unversioned)]
pub struct ContractTeamActionAlreadyCompletedError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    contract_id: Identifier,
    action_id: Identifier,
}

impl ContractTeamActionAlreadyCompletedError {
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

    /// The team action
    pub fn action_id(&self) -> Identifier {
        self.action_id
    }
}

impl From<ContractTeamActionAlreadyCompletedError> for ConsensusError {
    fn from(err: ContractTeamActionAlreadyCompletedError) -> Self {
        Self::StateError(StateError::ContractTeamActionAlreadyCompletedError(err))
    }
}
