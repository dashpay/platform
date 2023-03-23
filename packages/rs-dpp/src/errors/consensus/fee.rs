use thiserror::Error;

use crate::state_transition::fee::Credits;

#[derive(Error, Debug)]
pub enum FeeError {
    #[error("Current credits balance {balance} is not enough to pay {fee} fee")]
    BalanceIsNotEnoughError { balance: u64, fee: Credits },
}
