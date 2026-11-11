use crate::balances::credits::TokenAmount;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use thiserror::Error;

/// A token order's fill range is empty: a listing or an offer of less than its token's minimum
/// fill (`min` the minimum fill, `max` the order's amount), a tradeable token whose minimum fill is
/// 0 or above `i64::MAX` (`max`), or a fill whose minimum is 0 or above its maximum.
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
    "Token order fill range is invalid: the minimum {min} must be at least 1 and at most {max}"
)]
#[platform_serialize(unversioned)]
pub struct InvalidTokenOrderFillRangeError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    min: TokenAmount,
    max: TokenAmount,
}

impl InvalidTokenOrderFillRangeError {
    pub fn new(min: TokenAmount, max: TokenAmount) -> Self {
        Self { min, max }
    }

    pub fn min(&self) -> TokenAmount {
        self.min
    }

    pub fn max(&self) -> TokenAmount {
        self.max
    }
}

impl From<InvalidTokenOrderFillRangeError> for ConsensusError {
    fn from(err: InvalidTokenOrderFillRangeError) -> Self {
        Self::BasicError(BasicError::InvalidTokenOrderFillRangeError(err))
    }
}
