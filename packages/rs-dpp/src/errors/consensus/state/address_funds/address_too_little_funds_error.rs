use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use crate::fee::Credits;
use crate::identity::KeyOfType;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Insufficient address balance for key {key_of_type}: has {balance}, requires at least {required_balance}")]
#[platform_serialize(unversioned)]
pub struct AddressTooLittleFundsError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    key_of_type: KeyOfType,
    balance: Credits,
    required_balance: Credits,
}

impl AddressTooLittleFundsError {
    pub fn new(key_of_type: KeyOfType, balance: Credits, required_balance: Credits) -> Self {
        Self {
            key_of_type,
            balance,
            required_balance,
        }
    }

    pub fn key_of_type(&self) -> &KeyOfType {
        &self.key_of_type
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
