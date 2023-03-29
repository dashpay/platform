use crate::consensus::fee::balance_is_not_enough_error::BalanceIsNotEnoughError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FeeError {
    #[error(transparent)]
    BalanceIsNotEnoughError(BalanceIsNotEnoughError),
}
