use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use crate::identity::KeyID;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Duplicated public keys {duplicated_ids:?} found")]
#[platform_serialize(unversioned)]
pub struct DuplicatedIdentityPublicKeyBasicError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    duplicated_ids: Vec<KeyID>,
}

impl DuplicatedIdentityPublicKeyBasicError {
    pub fn new(duplicated_ids: Vec<KeyID>) -> Self {
        Self { duplicated_ids }
    }

    pub fn duplicated_public_keys_ids(&self) -> &Vec<KeyID> {
        &self.duplicated_ids
    }
}
impl From<DuplicatedIdentityPublicKeyBasicError> for ConsensusError {
    fn from(err: DuplicatedIdentityPublicKeyBasicError) -> Self {
        Self::BasicError(BasicError::DuplicatedIdentityPublicKeyBasicError(err))
    }
}
