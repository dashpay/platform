use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use thiserror::Error;

/// A document action's shielded token payment proves a different amount than the token cost
/// the document type requires for that action.
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
    "Document {} requires a token payment of {} of token {}, the shielded payment pays {}",
    action,
    required_amount,
    token_id,
    paid_amount
)]
#[platform_serialize(unversioned)]
pub struct TokenShieldedPaymentAmountMismatchError {
    token_id: Identifier,
    required_amount: u64,
    paid_amount: u64,
    action: String,
}

impl TokenShieldedPaymentAmountMismatchError {
    pub fn new(
        token_id: Identifier,
        required_amount: u64,
        paid_amount: u64,
        action: String,
    ) -> Self {
        Self {
            token_id,
            required_amount,
            paid_amount,
            action,
        }
    }

    pub fn token_id(&self) -> &Identifier {
        &self.token_id
    }

    pub fn required_amount(&self) -> u64 {
        self.required_amount
    }

    pub fn paid_amount(&self) -> u64 {
        self.paid_amount
    }

    pub fn action(&self) -> &str {
        &self.action
    }
}

impl From<TokenShieldedPaymentAmountMismatchError> for ConsensusError {
    fn from(err: TokenShieldedPaymentAmountMismatchError) -> Self {
        Self::StateError(StateError::TokenShieldedPaymentAmountMismatchError(err))
    }
}
