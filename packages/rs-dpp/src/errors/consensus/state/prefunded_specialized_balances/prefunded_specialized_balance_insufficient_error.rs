use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

use crate::prelude::Identifier;

use bincode::{Decode, Encode};

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error(
    "Insufficient specialized balance {balance_id} balance {balance} required {required_balance}"
)]
#[platform_serialize(unversioned)]
pub struct PrefundedSpecializedBalanceInsufficientError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    pub balance_id: Identifier,
    pub balance: u64,
    pub required_balance: u64,
}

impl PrefundedSpecializedBalanceInsufficientError {
    pub fn new(balance_id: Identifier, balance: u64, required_balance: u64) -> Self {
        Self {
            balance_id,
            balance,
            required_balance,
        }
    }

    pub fn balance_id(&self) -> &Identifier {
        &self.balance_id
    }

    pub fn balance(&self) -> u64 {
        self.balance
    }

    pub fn required_balance(&self) -> u64 {
        self.required_balance
    }
}
impl From<PrefundedSpecializedBalanceInsufficientError> for ConsensusError {
    fn from(err: PrefundedSpecializedBalanceInsufficientError) -> Self {
        Self::StateError(StateError::PrefundedSpecializedBalanceInsufficientError(
            err,
        ))
    }
}
