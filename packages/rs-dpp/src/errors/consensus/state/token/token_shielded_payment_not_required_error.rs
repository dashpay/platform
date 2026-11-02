use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use thiserror::Error;

/// A document transition carries a shielded token payment but the document type has no token
/// cost for that action, so there is nothing for the bundle to pay.
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
#[error(
    "Document {} carries a shielded payment of token {} but has no token cost to pay",
    action,
    token_id
)]
#[platform_serialize(unversioned)]
pub struct TokenShieldedPaymentNotRequiredError {
    token_id: Identifier,
    action: String,
}

impl TokenShieldedPaymentNotRequiredError {
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

impl From<TokenShieldedPaymentNotRequiredError> for ConsensusError {
    fn from(err: TokenShieldedPaymentNotRequiredError) -> Self {
        Self::StateError(StateError::TokenShieldedPaymentNotRequiredError(err))
    }
}
