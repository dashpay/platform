use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use thiserror::Error;

/// The token's marketplace trade mode is not `Tradeable`, so it cannot be listed or offered for,
/// and its open orders cannot be filled (they can still be cancelled).
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
#[error("Token {token_id} is not tradeable")]
#[platform_serialize(unversioned)]
pub struct TokenNotTradeableError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    token_id: Identifier,
}

impl TokenNotTradeableError {
    pub fn new(token_id: Identifier) -> Self {
        Self { token_id }
    }

    pub fn token_id(&self) -> Identifier {
        self.token_id
    }
}

impl From<TokenNotTradeableError> for ConsensusError {
    fn from(err: TokenNotTradeableError) -> Self {
        Self::StateError(StateError::TokenNotTradeableError(err))
    }
}
