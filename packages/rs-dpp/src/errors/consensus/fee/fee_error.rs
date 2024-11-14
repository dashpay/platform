use crate::errors::consensus::fee::balance_is_not_enough_error::BalanceIsNotEnoughError;
use crate::errors::consensus::ConsensusError;
use bincode::{Decode, Encode};
use thiserror::Error;

use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};

#[derive(
    Error, Debug, PartialEq, Encode, Decode, PlatformSerialize, PlatformDeserialize, Clone,
)]
pub enum FeeError {
    /*

    DO NOT CHANGE ORDER OF VARIANTS WITHOUT INTRODUCING OF NEW VERSION

    */
    #[error(transparent)]
    BalanceIsNotEnoughError(BalanceIsNotEnoughError),
}

impl From<FeeError> for ConsensusError {
    fn from(error: FeeError) -> Self {
        Self::FeeError(error)
    }
}
