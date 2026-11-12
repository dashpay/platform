use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use thiserror::Error;

/// The approval of a moderator's deletion of a settled document on an elected contract that has
/// no seated moderation team yet. Only the members of a seated team delete a settled document,
/// and the rule that says which of them must approve names its leader: no interim moderator
/// deletes one.
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
    "Contract {} has no seated moderation team, and only the members of one approve the deletion of a settled document",
    contract_id
)]
#[platform_serialize(unversioned)]
pub struct ContractModerationTeamNotSeatedError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    contract_id: Identifier,
}

impl ContractModerationTeamNotSeatedError {
    pub fn new(contract_id: Identifier) -> Self {
        Self { contract_id }
    }

    /// The moderated contract
    pub fn contract_id(&self) -> Identifier {
        self.contract_id
    }
}

impl From<ContractModerationTeamNotSeatedError> for ConsensusError {
    fn from(err: ContractModerationTeamNotSeatedError) -> Self {
        Self::StateError(StateError::ContractModerationTeamNotSeatedError(err))
    }
}
