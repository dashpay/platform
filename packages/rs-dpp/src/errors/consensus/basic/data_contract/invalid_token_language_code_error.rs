use crate::errors::consensus::basic::BasicError;
use crate::errors::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Invalid token language code: '{language_code}'")]
#[platform_serialize(unversioned)]
#[cfg_attr(feature = "apple", ferment_macro::export)]
pub struct InvalidTokenLanguageCodeError {
    /*
    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING A NEW VERSION
    */
    pub language_code: String,
}

impl InvalidTokenLanguageCodeError {
    pub fn new(language_code: String) -> Self {
        Self { language_code }
    }

    pub fn language_code(&self) -> &str {
        &self.language_code
    }
}

impl From<InvalidTokenLanguageCodeError> for ConsensusError {
    fn from(err: InvalidTokenLanguageCodeError) -> Self {
        Self::BasicError(BasicError::InvalidTokenLanguageCodeError(err))
    }
}
