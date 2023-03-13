use thiserror::Error;

use crate::prelude::{Fee, Balance};

#[derive(Error, Debug)]
pub enum FeeError {
    #[error("Current credits balance {balance} is not enough to pay {fee} fee")]
    BalanceIsNotEnoughError { balance: Balance, fee: Fee },
}
