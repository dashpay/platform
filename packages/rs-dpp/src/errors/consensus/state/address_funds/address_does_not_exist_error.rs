use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use crate::identity::KeyOfType;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[platform_serialize(unversioned)]
#[error("Address does not exist for key: {key_of_type:?}")]
pub struct AddressDoesNotExistError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    key_of_type: KeyOfType,
}

impl AddressDoesNotExistError {
    pub fn new(key_of_type: KeyOfType) -> Self {
        Self { key_of_type }
    }

    pub fn key_of_type(&self) -> &KeyOfType {
        &self.key_of_type
    }
}

impl From<AddressDoesNotExistError> for ConsensusError {
    fn from(err: AddressDoesNotExistError) -> Self {
        Self::StateError(StateError::AddressDoesNotExistError(err))
    }
}
