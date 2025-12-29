use crate::address_funds::PlatformAddress;
use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use crate::prelude::AddressNonce;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Invalid address nonce for {address}: expected {expected_nonce}, got {provided_nonce}")]
#[platform_serialize(unversioned)]
pub struct AddressInvalidNonceError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    address: PlatformAddress,
    provided_nonce: AddressNonce,
    expected_nonce: AddressNonce,
}

impl AddressInvalidNonceError {
    pub fn new(
        address: PlatformAddress,
        provided_nonce: AddressNonce,
        expected_nonce: AddressNonce,
    ) -> Self {
        Self {
            address,
            provided_nonce,
            expected_nonce,
        }
    }

    pub fn address(&self) -> &PlatformAddress {
        &self.address
    }

    pub fn provided_nonce(&self) -> AddressNonce {
        self.provided_nonce
    }

    pub fn expected_nonce(&self) -> AddressNonce {
        self.expected_nonce
    }
}

impl From<AddressInvalidNonceError> for ConsensusError {
    fn from(err: AddressInvalidNonceError) -> Self {
        Self::StateError(StateError::AddressInvalidNonceError(err))
    }
}
