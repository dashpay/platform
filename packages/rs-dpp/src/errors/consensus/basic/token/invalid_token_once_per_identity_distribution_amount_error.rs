use crate::balances::credits::TokenAmount;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use thiserror::Error;

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
    "Once-per-identity distribution amount {amount} is invalid: it must be between 1 and {max_amount}"
)]
#[platform_serialize(unversioned)]
pub struct InvalidTokenOncePerIdentityDistributionAmountError {
    amount: TokenAmount,
    max_amount: TokenAmount,
}

impl InvalidTokenOncePerIdentityDistributionAmountError {
    pub fn new(amount: TokenAmount, max_amount: TokenAmount) -> Self {
        Self { amount, max_amount }
    }

    pub fn amount(&self) -> TokenAmount {
        self.amount
    }

    pub fn max_amount(&self) -> TokenAmount {
        self.max_amount
    }
}

impl From<InvalidTokenOncePerIdentityDistributionAmountError> for ConsensusError {
    fn from(err: InvalidTokenOncePerIdentityDistributionAmountError) -> Self {
        Self::BasicError(BasicError::InvalidTokenOncePerIdentityDistributionAmountError(err))
    }
}
