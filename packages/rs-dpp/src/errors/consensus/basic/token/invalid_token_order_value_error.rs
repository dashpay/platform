use crate::balances::credits::TokenAmount;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::fee::Credits;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use thiserror::Error;

/// What a token order is worth in credits, its amount at its price and the decimals it signs,
/// does not fit in what an order can hold (`i64::MAX` credits).
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
#[error("A token order of {amount} base units at {price} credits per whole token ({decimals} decimals) is worth more credits than an order can hold")]
#[platform_serialize(unversioned)]
pub struct InvalidTokenOrderValueError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    amount: TokenAmount,
    price: Credits,
    decimals: u8,
}

impl InvalidTokenOrderValueError {
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

impl From<InvalidTokenOrderValueError> for ConsensusError {
    fn from(err: InvalidTokenOrderValueError) -> Self {
        Self::BasicError(BasicError::InvalidTokenOrderValueError(err))
    }
}
