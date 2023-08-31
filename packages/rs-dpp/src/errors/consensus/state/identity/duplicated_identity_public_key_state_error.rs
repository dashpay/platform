use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use crate::identity::KeyID;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Duplicated public keys {duplicated_public_key_ids:?} found")]
#[platform_serialize(unversioned)]
pub struct DuplicatedIdentityPublicKeyStateError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    duplicated_public_key_ids: Vec<KeyID>,
}

impl DuplicatedIdentityPublicKeyStateError {
    pub fn new(duplicated_public_key_ids: Vec<KeyID>) -> Self {
        Self {
            duplicated_public_key_ids,
        }
    }

    pub fn duplicated_public_key_ids(&self) -> &Vec<KeyID> {
        &self.duplicated_public_key_ids
    }
}
impl From<DuplicatedIdentityPublicKeyStateError> for ConsensusError {
    fn from(err: DuplicatedIdentityPublicKeyStateError) -> Self {
        Self::StateError(StateError::DuplicatedIdentityPublicKeyStateError(err))
    }
}
