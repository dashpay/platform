use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Insufficient funding amount: funding_amount={funding_amount} is less than minimum={minimum_funding_amount}")]
#[platform_serialize(unversioned)]
pub struct InsufficientFundingAmountError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    funding_amount: u64,
    minimum_funding_amount: u64,
}

impl InsufficientFundingAmountError {
    pub fn new(funding_amount: u64, minimum_funding_amount: u64) -> Self {
        Self {
            funding_amount,
            minimum_funding_amount,
        }
    }

    pub fn funding_amount(&self) -> u64 {
        self.funding_amount
    }

    pub fn minimum_funding_amount(&self) -> u64 {
        self.minimum_funding_amount
    }
}

impl From<InsufficientFundingAmountError> for ConsensusError {
    fn from(err: InsufficientFundingAmountError) -> Self {
        Self::BasicError(BasicError::InsufficientFundingAmountError(err))
    }
}
