use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use thiserror::Error;

/// An approval of the proposal to delete a settled document that changed since it was
/// proposed: its revision or its last modification moved, so the team would approve the
/// deletion of content it never saw. A member proposes afresh.
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
    "Document {} changed since team action {} on contract {} proposed its deletion",
    document_id,
    action_id,
    contract_id
)]
#[platform_serialize(unversioned)]
pub struct ContractTeamActionDocumentChangedError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    contract_id: Identifier,
    action_id: Identifier,
    document_id: Identifier,
}

impl ContractTeamActionDocumentChangedError {
    pub fn new(contract_id: Identifier, action_id: Identifier, document_id: Identifier) -> Self {
        Self {
            contract_id,
            action_id,
            document_id,
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

    /// The document the proposal names
    pub fn document_id(&self) -> Identifier {
        self.document_id
    }
}

impl From<ContractTeamActionDocumentChangedError> for ConsensusError {
    fn from(err: ContractTeamActionDocumentChangedError) -> Self {
        Self::StateError(StateError::ContractTeamActionDocumentChangedError(err))
    }
}
