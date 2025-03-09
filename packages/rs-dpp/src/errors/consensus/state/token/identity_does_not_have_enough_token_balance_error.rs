use crate::errors::consensus::state::state_error::StateError;
use crate::errors::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error(
    "Identity {} does not have enough balance for token {}: required {}, actual {}, action: {}",
    identity_id,
    token_id,
    required_balance,
    actual_balance,
    action
)]
#[platform_serialize(unversioned)]
#[cfg_attr(feature = "apple", ferment_macro::export)]
pub struct IdentityDoesNotHaveEnoughTokenBalanceError {
    pub token_id: Identifier,
    pub identity_id: Identifier,
    pub required_balance: u64,
    pub actual_balance: u64,
    pub action: String,
}

impl IdentityDoesNotHaveEnoughTokenBalanceError {
    pub fn new(
        token_id: Identifier,
        identity_id: Identifier,
        required_balance: u64,
        actual_balance: u64,
        action: String,
    ) -> Self {
        Self {
            token_id,
            identity_id,
            required_balance,
            actual_balance,
            action,
        }
    }
    pub fn token_id(&self) -> &Identifier {
        &self.token_id
    }

    pub fn identity_id(&self) -> &Identifier {
        &self.identity_id
    }

    pub fn required_balance(&self) -> u64 {
        self.required_balance
    }

    pub fn actual_balance(&self) -> u64 {
        self.actual_balance
    }

    pub fn action(&self) -> &str {
        &self.action
    }
}

impl From<IdentityDoesNotHaveEnoughTokenBalanceError> for ConsensusError {
    fn from(err: IdentityDoesNotHaveEnoughTokenBalanceError) -> Self {
        Self::StateError(StateError::IdentityDoesNotHaveEnoughTokenBalanceError(err))
    }
}
