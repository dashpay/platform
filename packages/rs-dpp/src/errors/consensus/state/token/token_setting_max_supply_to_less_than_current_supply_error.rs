use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error(
    "Token {token_id} attempted to set max supply to {max_supply}, which is less than the current supply {current_supply}"
)]
#[platform_serialize(unversioned)]
pub struct TokenSettingMaxSupplyToLessThanCurrentSupplyError {
    token_id: Identifier,
    max_supply: u64,
    current_supply: u64,
}

impl TokenSettingMaxSupplyToLessThanCurrentSupplyError {
    pub fn new(token_id: Identifier, max_supply: u64, current_supply: u64) -> Self {
        Self {
            token_id,
            max_supply,
            current_supply,
        }
    }

    pub fn token_id(&self) -> &Identifier {
        &self.token_id
    }

    pub fn max_supply(&self) -> u64 {
        self.max_supply
    }

    pub fn current_supply(&self) -> u64 {
        self.current_supply
    }
}

impl From<TokenSettingMaxSupplyToLessThanCurrentSupplyError> for ConsensusError {
    fn from(err: TokenSettingMaxSupplyToLessThanCurrentSupplyError) -> Self {
        Self::StateError(StateError::TokenSettingMaxSupplyToLessThanCurrentSupplyError(err))
    }
}
