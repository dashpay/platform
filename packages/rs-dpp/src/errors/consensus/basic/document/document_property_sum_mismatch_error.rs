use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use thiserror::Error;

/// A document supplied an object whose integer properties do not add up to the
/// `sumOfProperties` its type declares on that object.
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
#[error("The properties of {property} sum to {actual_sum}, but must sum to {expected_sum}")]
#[platform_serialize(unversioned)]
pub struct DocumentPropertySumMismatchError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    property: String,
    expected_sum: i64,
    actual_sum: i64,
}

impl DocumentPropertySumMismatchError {
    pub fn new(property: String, expected_sum: i64, actual_sum: i64) -> Self {
        Self {
            property,
            expected_sum,
            actual_sum,
        }
    }

    /// The dotted path of the object that declares the sum.
    pub fn property(&self) -> &str {
        &self.property
    }

    /// The declared sum.
    pub fn expected_sum(&self) -> i64 {
        self.expected_sum
    }

    /// What the object's properties add up to, clamped to the `i64` range.
    pub fn actual_sum(&self) -> i64 {
        self.actual_sum
    }
}

impl From<DocumentPropertySumMismatchError> for ConsensusError {
    fn from(err: DocumentPropertySumMismatchError) -> Self {
        Self::BasicError(BasicError::DocumentPropertySumMismatchError(err))
    }
}
