use crate::balances::credits::TokenAmount;
use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::fee::Credits;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use thiserror::Error;

/// What a token order is worth in credits, its amount at its price, does not fit in what Platform
/// can hold.
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
#[error("A token order of {amount} base units at {price} credits per whole token ({decimals} decimals) is worth more credits than Platform can hold")]
#[platform_serialize(unversioned)]
pub struct TokenOrderValueTooLargeError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    amount: TokenAmount,
    price: Credits,
    decimals: u8,
}

impl TokenOrderValueTooLargeError {
    pub fn new(amount: TokenAmount, price: Credits, decimals: u8) -> Self {
        Self {
            amount,
            price,
            decimals,
        }
    }

    pub fn amount(&self) -> TokenAmount {
        self.amount
    }

    pub fn price(&self) -> Credits {
        self.price
    }

    pub fn decimals(&self) -> u8 {
        self.decimals
    }
}

impl From<TokenOrderValueTooLargeError> for ConsensusError {
    fn from(err: TokenOrderValueTooLargeError) -> Self {
        Self::StateError(StateError::TokenOrderValueTooLargeError(err))
    }
}
