use crate::address_funds::PlatformAddress;
use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use crate::fee::Credits;
use crate::prelude::AddressNonce;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Insufficient combined address balances: total available is less than required {required_balance}")]
#[platform_serialize(unversioned)]
pub struct AddressesNotEnoughFundsError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    addresses_with_info: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    required_balance: Credits,
}

impl AddressesNotEnoughFundsError {
    pub fn new(
        addresses_with_info: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        required_balance: Credits,
    ) -> Self {
        Self {
            addresses_with_info,
            required_balance,
        }
    }

    pub fn addresses_with_info(&self) -> &BTreeMap<PlatformAddress, (AddressNonce, Credits)> {
        &self.addresses_with_info
    }

    pub fn required_balance(&self) -> Credits {
        self.required_balance
    }
}

impl From<AddressesNotEnoughFundsError> for ConsensusError {
    fn from(err: AddressesNotEnoughFundsError) -> Self {
        Self::StateError(StateError::AddressesNotEnoughFundsError(err))
    }
}
