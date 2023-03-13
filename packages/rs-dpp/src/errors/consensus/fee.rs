use thiserror::Error;

use crate::prelude::{Balance, Fee};

#[derive(Error, Debug)]
pub enum FeeError {
    #[error("Current credits balance {balance} is not enough to pay {fee} fee")]
    BalanceIsNotEnoughError { balance: Balance, fee: Fee },
}
