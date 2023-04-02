use crate::consensus::fee::fee_error::FeeError;
use crate::consensus::ConsensusError;
use crate::state_transition::fee::Credits;
use thiserror::Error;

use serde::{Deserialize, Serialize};

#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[error("Current credits balance {balance} is not enough to pay {fee} fee")]
pub struct BalanceIsNotEnoughError {
    balance: Credits,
    fee: Credits,
}

impl BalanceIsNotEnoughError {
    pub fn new(balance: Credits, fee: Credits) -> Self {
        Self { balance, fee }
    }

    pub fn balance(&self) -> Credits {
        self.balance
    }

    pub fn fee(&self) -> Credits {
        self.fee
    }
}

impl From<BalanceIsNotEnoughError> for ConsensusError {
    fn from(err: BalanceIsNotEnoughError) -> Self {
        Self::FeeError(FeeError::BalanceIsNotEnoughError(err))
    }
}
