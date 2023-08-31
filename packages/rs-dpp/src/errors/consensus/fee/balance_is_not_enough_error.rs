use crate::consensus::fee::fee_error::FeeError;
use crate::consensus::ConsensusError;
use crate::fee::Credits;
use thiserror::Error;

use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};

use bincode::{Decode, Encode};

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Current credits balance {balance} is not enough to pay {fee} fee")]
#[platform_serialize(unversioned)]
pub struct BalanceIsNotEnoughError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
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
