use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use thiserror::Error;

/// A second approval of an active team action by the same member of the seated team: its
/// proposal, or its earlier approval, already counts.
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
    "Moderator {} already approved team action {} on contract {}",
    signer_id,
    action_id,
    contract_id
)]
#[platform_serialize(unversioned)]
pub struct ContractTeamActionAlreadySignedError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    contract_id: Identifier,
    action_id: Identifier,
    signer_id: Identifier,
}

impl ContractTeamActionAlreadySignedError {
    pub fn new(contract_id: Identifier, action_id: Identifier, signer_id: Identifier) -> Self {
        Self {
            contract_id,
            action_id,
            signer_id,
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

    /// The member that approved again
    pub fn signer_id(&self) -> Identifier {
        self.signer_id
    }
}

impl From<ContractTeamActionAlreadySignedError> for ConsensusError {
    fn from(err: ContractTeamActionAlreadySignedError) -> Self {
        Self::StateError(StateError::ContractTeamActionAlreadySignedError(err))
    }
}
