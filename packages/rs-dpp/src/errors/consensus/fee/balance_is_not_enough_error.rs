use crate::consensus::fee::fee_error::FeeError;
use crate::consensus::fee_error::FeeError;
use crate::state_transition::fee::Credits;
use platform_value::Identifier;
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
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

impl From<BalanceIsNotEnoughError> for FeeError {
    fn from(err: BalanceIsNotEnoughError) -> Self {
        Self::BalanceIsNotEnoughError(err)
    }
}
