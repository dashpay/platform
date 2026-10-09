use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use thiserror::Error;

/// The token's configuration sets `transferable: false`, and the refused operation would move
/// it to another identity: a `TokenTransfer`, or a contract whose document type pays the token
/// to its own owner.
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
#[error("Token {token_id} is not transferable: {action} refused")]
#[platform_serialize(unversioned)]
pub struct TokenNotTransferableError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    token_id: Identifier,
    action: String,
}

impl TokenNotTransferableError {
    pub fn new(token_id: Identifier, action: String) -> Self {
        Self { token_id, action }
    }

    pub fn token_id(&self) -> &Identifier {
        &self.token_id
    }

    pub fn action(&self) -> &str {
        &self.action
    }
}

impl From<TokenNotTransferableError> for ConsensusError {
    fn from(err: TokenNotTransferableError) -> Self {
        Self::StateError(StateError::TokenNotTransferableError(err))
    }
}
