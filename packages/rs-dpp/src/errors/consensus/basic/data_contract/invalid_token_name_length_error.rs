use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Token {form} length was {actual}, but must be between {min} and {max} characters.")]
#[platform_serialize(unversioned)]
pub struct InvalidTokenNameLengthError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING A NEW VERSION

    */
    actual: usize,
    min: usize,
    max: usize,
    form: String,
}

impl InvalidTokenNameLengthError {
    pub fn new(actual: usize, min: usize, max: usize, form: impl Into<String>) -> Self {
        Self {
            actual,
            min,
            max,
            form: form.into(),
        }
    }

    pub fn actual(&self) -> usize {
        self.actual
    }

    pub fn min(&self) -> usize {
        self.min
    }

    pub fn max(&self) -> usize {
        self.max
    }

    pub fn form(&self) -> &str {
        &self.form
    }
}

impl From<InvalidTokenNameLengthError> for ConsensusError {
    fn from(err: InvalidTokenNameLengthError) -> Self {
        ConsensusError::BasicError(BasicError::InvalidTokenNameLengthError(err))
    }
}
