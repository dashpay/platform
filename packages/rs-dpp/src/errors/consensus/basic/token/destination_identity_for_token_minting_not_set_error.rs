use crate::errors::consensus::basic::BasicError;
use crate::errors::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Destination identity for minting not set for token {}", token_id)]
#[platform_serialize(unversioned)]
#[cfg_attr(feature = "apple", ferment_macro::export)]
pub struct DestinationIdentityForTokenMintingNotSetError {
    pub token_id: Identifier,
}

impl DestinationIdentityForTokenMintingNotSetError {
    pub fn new(token_id: Identifier) -> Self {
        Self { token_id }
    }
    pub fn token_id(&self) -> Identifier {
        self.token_id
    }
}

impl From<DestinationIdentityForTokenMintingNotSetError> for ConsensusError {
    fn from(err: DestinationIdentityForTokenMintingNotSetError) -> Self {
        Self::BasicError(BasicError::DestinationIdentityForTokenMintingNotSetError(
            err,
        ))
    }
}
