use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use thiserror::Error;

/// A second approval by the same moderator of the deletion of a settled document, while the
/// approvals it is among are still open: each member of the team approves once.
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
    "Moderator {} already approved the deletion of settled document {} on contract {}",
    moderator_id,
    document_id,
    contract_id
)]
#[platform_serialize(unversioned)]
pub struct SettledDeletionAlreadyApprovedError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    contract_id: Identifier,
    document_id: Identifier,
    moderator_id: Identifier,
}

impl SettledDeletionAlreadyApprovedError {
    pub fn new(contract_id: Identifier, document_id: Identifier, moderator_id: Identifier) -> Self {
        Self {
            contract_id,
            document_id,
            moderator_id,
        }
    }

    /// The moderated contract
    pub fn contract_id(&self) -> Identifier {
        self.contract_id
    }

    /// The document the deletion names
    pub fn document_id(&self) -> Identifier {
        self.document_id
    }

    /// The moderator that approved again
    pub fn moderator_id(&self) -> Identifier {
        self.moderator_id
    }
}

impl From<SettledDeletionAlreadyApprovedError> for ConsensusError {
    fn from(err: SettledDeletionAlreadyApprovedError) -> Self {
        Self::StateError(StateError::SettledDeletionAlreadyApprovedError(err))
    }
}
