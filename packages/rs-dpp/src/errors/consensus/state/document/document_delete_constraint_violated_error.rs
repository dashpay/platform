use crate::consensus::basic::document::PropertyConstraintViolation;
use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use thiserror::Error;

/// The owner's delete of a document breaks a rule of its document type's
/// `deleteConstraints` (protocol version 14): judged on the stored document,
/// the rule does not hold, or evaluating it overflowed, divided by zero, raised
/// to a negative power or read a value that is not an integer. A rule reads
/// state (the stored document, and a `countOf` or `sumOf` total as it will be
/// once the document is gone), so the refusal is a state error, paid.
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
    "Document {document_id} of type \"{document_type_name}\" can not be deleted: it breaks \
     its deleteConstraints rule \"{constraint}\": {violation}"
)]
#[platform_serialize(unversioned)]
pub struct DocumentDeleteConstraintViolatedError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    document_id: Identifier,
    document_type_name: String,
    /// The name of the broken rule, its key in `deleteConstraints`.
    constraint: String,
    violation: PropertyConstraintViolation,
}

impl DocumentDeleteConstraintViolatedError {
    pub fn new(
        document_id: Identifier,
        document_type_name: String,
        constraint: String,
        violation: PropertyConstraintViolation,
    ) -> Self {
        Self {
            document_id,
            document_type_name,
            constraint,
            violation,
        }
    }

    pub fn document_id(&self) -> &Identifier {
        &self.document_id
    }

    pub fn document_type_name(&self) -> &str {
        &self.document_type_name
    }

    pub fn constraint(&self) -> &str {
        &self.constraint
    }

    pub fn violation(&self) -> PropertyConstraintViolation {
        self.violation
    }
}

impl From<DocumentDeleteConstraintViolatedError> for ConsensusError {
    fn from(err: DocumentDeleteConstraintViolatedError) -> Self {
        Self::StateError(StateError::DocumentDeleteConstraintViolatedError(err))
    }
}
