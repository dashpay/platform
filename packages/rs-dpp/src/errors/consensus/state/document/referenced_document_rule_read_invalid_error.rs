use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use thiserror::Error;

/// A `propertyConstraints` rule reads, through a `refersTo` into a document
/// type of another contract (`{ "referenced": [reference, field] }`), a field
/// that type can not give it: one it does not have, a transient one, one of
/// another kind than the rule reads, a time or height it does not record, a
/// `$creatorId` it does not record, a constant its `enum` does not list, or,
/// through a `moderatedDocument` reference, a field its removal record does not
/// keep. Reported at contract registration and update, against the stored
/// contract; a rule reading a document type of the declaring contract is
/// refused by the contract parse instead.
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
    "propertyConstraints rule {rule} of {document_type_name} reads through {reference}: {reason}"
)]
#[platform_serialize(unversioned)]
pub struct ReferencedDocumentRuleReadInvalidError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    document_type_name: String,
    rule: String,
    reference: String,
    reason: String,
}

impl ReferencedDocumentRuleReadInvalidError {
    pub fn new(
        document_type_name: String,
        rule: String,
        reference: String,
        reason: String,
    ) -> Self {
        Self {
            document_type_name,
            rule,
            reference,
            reason,
        }
    }

    /// The document type declaring the rule.
    pub fn document_type_name(&self) -> &str {
        &self.document_type_name
    }

    /// The rule's name.
    pub fn rule(&self) -> &str {
        &self.rule
    }

    /// The reference property the rule reads through.
    pub fn reference(&self) -> &str {
        &self.reference
    }

    /// What the referenced document type can not give the rule.
    pub fn reason(&self) -> &str {
        &self.reason
    }
}

impl From<ReferencedDocumentRuleReadInvalidError> for ConsensusError {
    fn from(err: ReferencedDocumentRuleReadInvalidError) -> Self {
        Self::StateError(StateError::ReferencedDocumentRuleReadInvalidError(err))
    }
}
