use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use thiserror::Error;

/// A decision proposal (protocol version 15) whose rule makes no yes/no poll the system limits
/// allow (11101): a `passPercent` below 50 or not below 100, a `minimumTurnoutPercent` above the
/// share of the network a poll may require, or either missing. The contract's schema refuses
/// these first, so this is what remains if the schema and the poll limits ever disagree.
/// `reason` says which.
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
#[error("decision proposal {document_id} has an invalid rule: {reason}")]
#[platform_serialize(unversioned)]
pub struct InvalidDecisionProposalRuleError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    document_id: Identifier,
    reason: String,
}

impl InvalidDecisionProposalRuleError {
    pub fn new(document_id: Identifier, reason: String) -> Self {
        Self {
            document_id,
            reason,
        }
    }

    pub fn document_id(&self) -> Identifier {
        self.document_id
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }
}

impl From<InvalidDecisionProposalRuleError> for ConsensusError {
    fn from(err: InvalidDecisionProposalRuleError) -> Self {
        Self::BasicError(BasicError::InvalidDecisionProposalRuleError(err))
    }
}
