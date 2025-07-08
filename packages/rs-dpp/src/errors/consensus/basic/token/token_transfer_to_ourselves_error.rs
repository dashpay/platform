use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error(
    "Token transfer to the same identity is not allowed. Token ID: {}, Identity ID: {}",
    token_id,
    identity_id
)]
#[platform_serialize(unversioned)]
pub struct TokenTransferToOurselfError {
    token_id: Identifier,
    identity_id: Identifier,
}

impl TokenTransferToOurselfError {
    pub fn new(token_id: Identifier, identity_id: Identifier) -> Self {
        Self {
            token_id,
            identity_id,
        }
    }

    pub fn token_id(&self) -> &Identifier {
        &self.token_id
    }

    pub fn identity_id(&self) -> &Identifier {
        &self.identity_id
    }
}

impl From<TokenTransferToOurselfError> for ConsensusError {
    fn from(err: TokenTransferToOurselfError) -> Self {
        Self::BasicError(BasicError::TokenTransferToOurselfError(err))
    }
}
