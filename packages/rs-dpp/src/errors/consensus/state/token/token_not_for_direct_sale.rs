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
#[error("Token {token_id} is not available for direct sale.")]
#[platform_serialize(unversioned)]
pub struct TokenNotForDirectSale {
    token_id: Identifier,
}

impl TokenNotForDirectSale {
    pub fn new(token_id: Identifier) -> Self {
        Self { token_id }
    }

    pub fn token_id(&self) -> &Identifier {
        &self.token_id
    }
}

impl From<TokenNotForDirectSale> for ConsensusError {
    fn from(err: TokenNotForDirectSale) -> Self {
        Self::StateError(StateError::TokenNotForDirectSale(err))
    }
}
