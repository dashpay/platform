use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use std::fmt;
use thiserror::Error;

/// Why a document breaks a rule of its type's `propertyConstraints`.
///
/// Encoded by position in consensus errors: a new reason goes at the end.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub enum PropertyConstraintViolation {
    /// Both sides of the rule evaluate, but they do not compare as it requires.
    NotMet,
    /// A value the rule reads, or a result it computes on the way, does not fit
    /// a 128-bit signed integer.
    Overflow,
    /// A `divide` or `modulo` whose divisor evaluates to 0.
    DivisionByZero,
    /// A `power` whose exponent evaluates to a negative number, which has no
    /// integer result.
    NegativeExponent,
    /// A value the rule reads is not an integer: a float with no fractional
    /// part, which the schema's `integer` type admits but no integer property
    /// can store.
    NotAnInteger,
}

impl fmt::Display for PropertyConstraintViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NotMet => "its two sides do not compare as it requires",
            Self::Overflow => "a value it reads or computes does not fit a 128-bit signed integer",
            Self::DivisionByZero => "it divides by zero",
            Self::NegativeExponent => "it raises to a negative power",
            Self::NotAnInteger => "a value it reads is not an integer",
        })
    }
}

/// A created or replaced document breaks a rule of its document type's
/// `propertyConstraints`: the comparison does not hold, or evaluating it
/// overflowed, divided by zero, raised to a negative power or read a value that
/// is not an integer.
///
/// A pure structure check on document create and replace (protocol version 14):
/// it reads the transition alone, so it is a basic error, not a state one.
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
    "A document of type \"{document_type_name}\" breaks its propertyConstraints rule \
     \"{constraint}\": {violation}"
)]
#[platform_serialize(unversioned)]
pub struct DocumentPropertyConstraintViolatedError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    document_type_name: String,
    /// The name of the broken rule, its key in `propertyConstraints`.
    constraint: String,
    violation: PropertyConstraintViolation,
}

impl DocumentPropertyConstraintViolatedError {
    pub fn new(
        document_type_name: String,
        constraint: String,
        violation: PropertyConstraintViolation,
    ) -> Self {
        Self {
            document_type_name,
            constraint,
            violation,
        }
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

impl From<DocumentPropertyConstraintViolatedError> for ConsensusError {
    fn from(err: DocumentPropertyConstraintViolatedError) -> Self {
        Self::BasicError(BasicError::DocumentPropertyConstraintViolatedError(err))
    }
}
