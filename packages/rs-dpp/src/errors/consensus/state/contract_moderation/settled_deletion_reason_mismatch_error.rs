use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::data_contract::config::moderation::ContractModerationReason;
use crate::errors::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use thiserror::Error;

/// The approval of a moderator's deletion of a settled document whose reason is not the one the
/// open approvals of that deletion hold. Every approval of one deletion is for the same reason,
/// the first's: a moderator approves what the others approved.
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
    "The approvals of the deletion of settled document {} on contract {} are for another reason: {}",
    document_id,
    contract_id,
    approved_reason.text
)]
#[platform_serialize(unversioned)]
pub struct SettledDeletionReasonMismatchError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    contract_id: Identifier,
    document_id: Identifier,
    approved_reason: ContractModerationReason,
}

impl SettledDeletionReasonMismatchError {
    pub fn new(
        contract_id: Identifier,
        document_id: Identifier,
        approved_reason: ContractModerationReason,
    ) -> Self {
        Self {
            contract_id,
            document_id,
            approved_reason,
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

    /// The reason the open approvals hold, which an approval must repeat
    pub fn approved_reason(&self) -> &ContractModerationReason {
        &self.approved_reason
    }
}

impl From<SettledDeletionReasonMismatchError> for ConsensusError {
    fn from(err: SettledDeletionReasonMismatchError) -> Self {
        Self::StateError(StateError::SettledDeletionReasonMismatchError(err))
    }
}
