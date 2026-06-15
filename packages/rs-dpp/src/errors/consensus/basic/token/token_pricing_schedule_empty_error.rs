use crate::consensus::basic::BasicError;
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
    "Token {} pricing schedule cannot be set to an empty set of prices",
    token_id
)]
#[platform_serialize(unversioned)]
pub struct TokenPricingScheduleEmptyError {
    token_id: Identifier,
}

impl TokenPricingScheduleEmptyError {
    /// Creates a new `TokenPricingScheduleEmptyError`.
    pub fn new(token_id: Identifier) -> Self {
        Self { token_id }
    }

    /// Returns the identifier of the token whose pricing schedule was empty.
    pub fn token_id(&self) -> Identifier {
        self.token_id
    }
}

impl From<TokenPricingScheduleEmptyError> for ConsensusError {
    fn from(err: TokenPricingScheduleEmptyError) -> Self {
        Self::BasicError(BasicError::TokenPricingScheduleEmptyError(err))
    }
}
