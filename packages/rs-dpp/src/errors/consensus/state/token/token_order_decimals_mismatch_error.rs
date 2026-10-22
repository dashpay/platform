use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use thiserror::Error;

/// A token order was signed at a price scale (the token's decimals) other than the token's when
/// it executes. An order's price is per whole token, so the decimals set what it is worth: the
/// order is refused rather than priced at a scale its signer did not sign.
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
#[error("A token order for token {token_id} was signed at {signed_decimals} decimals, but the token has {token_decimals}")]
#[platform_serialize(unversioned)]
pub struct TokenOrderDecimalsMismatchError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    token_id: Identifier,
    signed_decimals: u8,
    token_decimals: u8,
}

impl TokenOrderDecimalsMismatchError {
    pub fn new(token_id: Identifier, signed_decimals: u8, token_decimals: u8) -> Self {
        Self {
            token_id,
            signed_decimals,
            token_decimals,
        }
    }

    pub fn token_id(&self) -> Identifier {
        self.token_id
    }

    pub fn signed_decimals(&self) -> u8 {
        self.signed_decimals
    }

    pub fn token_decimals(&self) -> u8 {
        self.token_decimals
    }
}

impl From<TokenOrderDecimalsMismatchError> for ConsensusError {
    fn from(err: TokenOrderDecimalsMismatchError) -> Self {
        Self::StateError(StateError::TokenOrderDecimalsMismatchError(err))
    }
}
