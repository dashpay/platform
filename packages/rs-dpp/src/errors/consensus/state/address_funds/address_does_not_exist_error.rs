use crate::address_funds::PlatformAddress;
use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[platform_serialize(unversioned)]
#[error("Address does not exist: {address}")]
pub struct AddressDoesNotExistError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    address: PlatformAddress,
}

impl AddressDoesNotExistError {
    pub fn new(address: PlatformAddress) -> Self {
        Self { address }
    }

    pub fn address(&self) -> &PlatformAddress {
        &self.address
    }
}

impl From<AddressDoesNotExistError> for ConsensusError {
    fn from(err: AddressDoesNotExistError) -> Self {
        Self::StateError(StateError::AddressDoesNotExistError(err))
    }
}
