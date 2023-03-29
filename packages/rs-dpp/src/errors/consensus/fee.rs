use thiserror::Error;
use crate::consensus::balance_is_not_enough_error::BalanceIsNotEnoughError;

use crate::state_transition::fee::Credits;

#[derive(Error, Debug)]
pub enum FeeError {
    #[error(transparent)]
    BalanceIsNotEnoughError(BalanceIsNotEnoughError),
}
