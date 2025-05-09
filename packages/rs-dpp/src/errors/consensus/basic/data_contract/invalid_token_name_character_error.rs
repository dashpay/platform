use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Data contract has a token name {token_name} in {form} with invalid characters. Token names must not contain whitespace or control characters.")]
#[platform_serialize(unversioned)]
pub struct InvalidTokenNameCharacterError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    form: String,
    token_name: String,
}

impl InvalidTokenNameCharacterError {
    pub fn new(form: String, token_name: String) -> Self {
        Self { form, token_name }
    }

    pub fn form(&self) -> &str {
        &self.form
    }

    pub fn token_name(&self) -> &str {
        &self.token_name
    }
}

impl From<InvalidTokenNameCharacterError> for ConsensusError {
    fn from(err: InvalidTokenNameCharacterError) -> Self {
        Self::BasicError(BasicError::InvalidTokenNameCharacterError(err))
    }
}
