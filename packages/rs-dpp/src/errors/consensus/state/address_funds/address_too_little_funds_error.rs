use crate::address_funds::PlatformAddress;
use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use crate::fee::Credits;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Insufficient address balance for {address}: has {balance}, requires at least {required_balance}")]
#[platform_serialize(unversioned)]
pub struct AddressTooLittleFundsError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    address: PlatformAddress,
    balance: Credits,
    required_balance: Credits,
}

impl AddressTooLittleFundsError {
    pub fn new(address: PlatformAddress, balance: Credits, required_balance: Credits) -> Self {
        Self {
            address,
            balance,
            required_balance,
        }
    }

    pub fn address(&self) -> &PlatformAddress {
        &self.address
    }

    pub fn balance(&self) -> Credits {
        self.balance
    }

    pub fn required_balance(&self) -> Credits {
        self.required_balance
    }
}

impl From<AddressTooLittleFundsError> for ConsensusError {
    fn from(err: AddressTooLittleFundsError) -> Self {
        Self::StateError(StateError::AddressTooLittleFundsError(err))
    }
}
