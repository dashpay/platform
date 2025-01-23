use crate::balances::credits::TokenAmount;
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
    "Token {token_id} attempted to mint {amount}, which exceeds the max supply {max_supply}, current supply is {current_supply}"
)]
#[platform_serialize(unversioned)]
pub struct TokenMintPastMaxSupplyError {
    token_id: Identifier,
    amount: TokenAmount,
    current_supply: TokenAmount,
    max_supply: TokenAmount,
}

impl TokenMintPastMaxSupplyError {
    pub fn new(
        token_id: Identifier,
        amount: TokenAmount,
        current_supply: TokenAmount,
        max_supply: TokenAmount,
    ) -> Self {
        Self {
            token_id,
            amount,
            current_supply,
            max_supply,
        }
    }

    pub fn token_id(&self) -> &Identifier {
        &self.token_id
    }

    pub fn amount(&self) -> TokenAmount {
        self.amount
    }

    pub fn current_supply(&self) -> TokenAmount {
        self.current_supply
    }

    pub fn max_supply(&self) -> TokenAmount {
        self.max_supply
    }
}

impl From<TokenMintPastMaxSupplyError> for ConsensusError {
    fn from(err: TokenMintPastMaxSupplyError) -> Self {
        Self::StateError(StateError::TokenMintPastMaxSupplyError(err))
    }
}
