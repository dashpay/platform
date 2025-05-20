use crate::errors::consensus::basic::BasicError;
use crate::errors::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Token note is only allowed when the signer is the proposer")]
#[platform_serialize(unversioned)]
#[cfg_attr(feature = "apple", ferment_macro::export)]
pub struct TokenNoteOnlyAllowedWhenProposerError;

impl TokenNoteOnlyAllowedWhenProposerError {
    /// Creates a new `TokenNoteOnlyAllowedWhenProposerError`.
    pub fn new() -> Self {
        Self
    }
}

impl Default for TokenNoteOnlyAllowedWhenProposerError {
    fn default() -> Self {
        Self::new()
    }
}

impl From<TokenNoteOnlyAllowedWhenProposerError> for ConsensusError {
    fn from(err: TokenNoteOnlyAllowedWhenProposerError) -> Self {
        Self::BasicError(BasicError::TokenNoteOnlyAllowedWhenProposerError(err))
    }
}
