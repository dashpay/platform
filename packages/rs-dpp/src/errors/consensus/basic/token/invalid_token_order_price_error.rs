use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::fee::Credits;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use thiserror::Error;

/// A token order's price is the credits per whole token: at least 1 credit, and at most what
/// an order's signed 8-byte fields hold.
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
#[error("Token order price {price} must be between 1 and {max_price} credits per whole token")]
#[platform_serialize(unversioned)]
pub struct InvalidTokenOrderPriceError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    price: Credits,
    max_price: Credits,
}

impl InvalidTokenOrderPriceError {
    pub fn new(price: Credits, max_price: Credits) -> Self {
        Self { price, max_price }
    }

    pub fn price(&self) -> Credits {
        self.price
    }

    pub fn max_price(&self) -> Credits {
        self.max_price
    }
}

impl From<InvalidTokenOrderPriceError> for ConsensusError {
    fn from(err: InvalidTokenOrderPriceError) -> Self {
        Self::BasicError(BasicError::InvalidTokenOrderPriceError(err))
    }
}
