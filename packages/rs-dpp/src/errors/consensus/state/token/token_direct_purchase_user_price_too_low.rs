use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::fee::Credits;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error(
    "Provided direct purchase price {user_price} is below the required price {required_price} for token {token_id}."
)]
#[platform_serialize(unversioned)]
pub struct TokenDirectPurchaseUserPriceTooLow {
    token_id: Identifier,
    user_price: Credits,
    required_price: Credits,
}

impl TokenDirectPurchaseUserPriceTooLow {
    pub fn new(token_id: Identifier, user_price: Credits, required_price: Credits) -> Self {
        Self {
            token_id,
            user_price,
            required_price,
        }
    }

    pub fn token_id(&self) -> &Identifier {
        &self.token_id
    }

    pub fn user_price(&self) -> Credits {
        self.user_price
    }

    pub fn required_price(&self) -> Credits {
        self.required_price
    }
}

impl From<TokenDirectPurchaseUserPriceTooLow> for ConsensusError {
    fn from(err: TokenDirectPurchaseUserPriceTooLow) -> Self {
        Self::StateError(StateError::TokenDirectPurchaseUserPriceTooLow(err))
    }
}
