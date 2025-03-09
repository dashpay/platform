use crate::errors::consensus::state::state_error::StateError;
use crate::errors::consensus::ConsensusError;
use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use thiserror::Error;

use bincode::{Decode, Encode};

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Insufficient identity {identity_id} balance {balance} required {required_balance}")]
#[platform_serialize(unversioned)]
#[cfg_attr(feature = "apple", ferment_macro::export)]
pub struct IdentityInsufficientBalanceError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    pub identity_id: Identifier,
    pub balance: u64,
    pub required_balance: u64,
}

impl IdentityInsufficientBalanceError {
    pub fn new(identity_id: Identifier, balance: u64, required_balance: u64) -> Self {
        Self {
            identity_id,
            balance,
            required_balance,
        }
    }

    pub fn identity_id(&self) -> &Identifier {
        &self.identity_id
    }

    pub fn balance(&self) -> u64 {
        self.balance
    }

    pub fn required_balance(&self) -> u64 {
        self.required_balance
    }
}
impl From<IdentityInsufficientBalanceError> for ConsensusError {
    fn from(err: IdentityInsufficientBalanceError) -> Self {
        Self::StateError(StateError::IdentityInsufficientBalanceError(err))
    }
}
