use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use thiserror::Error;

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
#[error("The {field} of the moderation charter is malformed: {reason}")]
#[platform_serialize(unversioned)]
pub struct ModerationCharterMalformedFieldError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    field: String,
    reason: String,
}

impl ModerationCharterMalformedFieldError {
    pub fn new(field: String, reason: String) -> Self {
        Self { field, reason }
    }

    /// The error for the charter property `field`, with `reason` said any way.
    pub fn for_field(field: &str, reason: impl Into<String>) -> Self {
        Self {
            field: field.to_string(),
            reason: reason.into(),
        }
    }

    /// The charter property that is malformed
    pub fn field(&self) -> &str {
        &self.field
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }
}

impl From<ModerationCharterMalformedFieldError> for ConsensusError {
    fn from(err: ModerationCharterMalformedFieldError) -> Self {
        Self::BasicError(BasicError::ModerationCharterMalformedFieldError(err))
    }
}
