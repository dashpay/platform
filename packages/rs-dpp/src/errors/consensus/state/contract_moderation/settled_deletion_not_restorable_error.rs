use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use crate::identity::TimestampMillis;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use thiserror::Error;

/// A restore of a document the contract's seated moderation team deleted together, its approvals
/// meeting its type's `moderatorAbilities.deleteSettled` rule: no single moderator undoes what
/// the leader and the members agreed on, so the deletion stands.
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
    "Document {} on contract {} was deleted at {} by the approvals of the seated moderation team, and a deletion the team agreed on is not restored",
    document_id,
    contract_id,
    deleted_at
)]
#[platform_serialize(unversioned)]
pub struct SettledDeletionNotRestorableError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    contract_id: Identifier,
    document_id: Identifier,
    deleted_at: TimestampMillis,
}

impl SettledDeletionNotRestorableError {
    pub fn new(
        contract_id: Identifier,
        document_id: Identifier,
        deleted_at: TimestampMillis,
    ) -> Self {
        Self {
            contract_id,
            document_id,
            deleted_at,
        }
    }

    /// The moderated contract
    pub fn contract_id(&self) -> Identifier {
        self.contract_id
    }

    /// The document the restore names
    pub fn document_id(&self) -> Identifier {
        self.document_id
    }

    /// The time of the block whose approval deleted the document, in milliseconds
    pub fn deleted_at(&self) -> TimestampMillis {
        self.deleted_at
    }
}

impl From<SettledDeletionNotRestorableError> for ConsensusError {
    fn from(err: SettledDeletionNotRestorableError) -> Self {
        Self::StateError(StateError::SettledDeletionNotRestorableError(err))
    }
}
