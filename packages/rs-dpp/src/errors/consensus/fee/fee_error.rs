use crate::errors::consensus::fee::balance_is_not_enough_error::BalanceIsNotEnoughError;
use crate::errors::consensus::ConsensusError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FeeError {
    #[error(transparent)]
    BalanceIsNotEnoughError(BalanceIsNotEnoughError),
}

impl From<FeeError> for ConsensusError {
    fn from(error: FeeError) -> Self {
        Self::FeeError(error)
    }
}
