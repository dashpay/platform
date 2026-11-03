use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use thiserror::Error;

/// A ban, a suspension, a warning or a document deletion by a member of a contract's seated
/// moderation team whose reason names no reason document, or one the team's proposal does not
/// list. Every such action of a seated team names a `reason` document of the moderation
/// charters contract that its proposal lists; a proposal that lists none can take no such
/// action. Lifting a ban, a suspension or warnings and restoring a document carry no reason
/// and are not checked, and neither are the moderators a declaration names.
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
    "The moderation of contract {} names {}, which the proposal {} of its seated team does not list",
    contract_id,
    reason_document_id.map(|id| format!("reason document {}", id)).unwrap_or_else(|| "no reason document".to_string()),
    submitted_charter_id
)]
#[platform_serialize(unversioned)]
pub struct ModerationReasonNotListedError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    contract_id: Identifier,
    submitted_charter_id: Identifier,
    reason_document_id: Option<Identifier>,
}

impl ModerationReasonNotListedError {
    pub fn new(
        contract_id: Identifier,
        submitted_charter_id: Identifier,
        reason_document_id: Option<Identifier>,
    ) -> Self {
        Self {
            contract_id,
            submitted_charter_id,
            reason_document_id,
        }
    }

    /// The moderated contract
    pub fn contract_id(&self) -> Identifier {
        self.contract_id
    }

    /// The proposal the seated team runs on, whose `reasons` the action is checked against
    pub fn submitted_charter_id(&self) -> Identifier {
        self.submitted_charter_id
    }

    /// The reason document the action named, `None` when it named none
    pub fn reason_document_id(&self) -> Option<Identifier> {
        self.reason_document_id
    }
}

impl From<ModerationReasonNotListedError> for ConsensusError {
    fn from(err: ModerationReasonNotListedError) -> Self {
        Self::StateError(StateError::ModerationReasonNotListedError(err))
    }
}
