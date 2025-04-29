use crate::errors::consensus::state::state_error::StateError;
use crate::errors::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error(
    "Requested token amount {desired_amount} is below the minimum sale amount {minimum_amount} for token {token_id}."
)]
#[platform_serialize(unversioned)]
#[cfg_attr(feature = "apple", ferment_macro::export)]
pub struct TokenAmountUnderMinimumSaleAmount {
    pub token_id: Identifier,
    pub desired_amount: u64,
    pub minimum_amount: u64,
}

impl TokenAmountUnderMinimumSaleAmount {
    pub fn new(token_id: Identifier, desired_amount: u64, minimum_amount: u64) -> Self {
        Self {
            token_id,
            desired_amount,
            minimum_amount,
        }
    }

    pub fn token_id(&self) -> &Identifier {
        &self.token_id
    }

    pub fn desired_amount(&self) -> u64 {
        self.desired_amount
    }

    pub fn minimum_amount(&self) -> u64 {
        self.minimum_amount
    }
}

impl From<TokenAmountUnderMinimumSaleAmount> for ConsensusError {
    fn from(err: TokenAmountUnderMinimumSaleAmount) -> Self {
        Self::StateError(StateError::TokenAmountUnderMinimumSaleAmount(err))
    }
}
