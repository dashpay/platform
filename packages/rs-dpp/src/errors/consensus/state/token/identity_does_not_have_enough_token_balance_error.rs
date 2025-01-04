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
    "Identity {} does not have enough token balance: required {}, actual {}, action: {}",
    identity_id,
    required_balance,
    actual_balance,
    action
)]
#[platform_serialize(unversioned)]
pub struct IdentityDoesNotHaveEnoughTokenBalanceError {
    identity_id: Identifier,
    required_balance: u64,
    actual_balance: u64,
    action: String,
}

impl IdentityDoesNotHaveEnoughTokenBalanceError {
    pub fn new(
        identity_id: Identifier,
        required_balance: u64,
        actual_balance: u64,
        action: String,
    ) -> Self {
        Self {
            identity_id,
            required_balance,
            actual_balance,
            action,
        }
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
