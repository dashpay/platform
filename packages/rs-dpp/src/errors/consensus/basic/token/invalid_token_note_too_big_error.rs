use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error(
    "Invalid token note: '{}' is too long ({} bytes), max allowed is {} bytes",
    note_type,
    note_length,
    max_note_length
)]
#[platform_serialize(unversioned)]
pub struct InvalidTokenNoteTooBigError {
    max_note_length: u32,
    note_type: String,
    note_length: u32,
}

impl InvalidTokenNoteTooBigError {
    /// Creates a new `InvalidTokenNoteTooBigError`.
    pub fn new(max_note_length: u32, note_type: &str, note_length: u32) -> Self {
        Self {
            max_note_length,
            note_type: note_type.to_string(),
            note_length,
        }
    }

    /// Returns the maximum allowed note length.
    pub fn max_note_length(&self) -> u32 {
        self.max_note_length
    }

    /// Returns the type of note that exceeded the allowed length.
    pub fn note_type(&self) -> &str {
        &self.note_type
    }

    /// Returns the actual note length that was too large.
    pub fn note_length(&self) -> u32 {
        self.note_length
    }
}

impl From<InvalidTokenNoteTooBigError> for ConsensusError {
    fn from(err: InvalidTokenNoteTooBigError) -> Self {
        Self::BasicError(BasicError::InvalidTokenNoteTooBigError(err))
    }
}
