use crate::errors::consensus::state::state_error::StateError;
use crate::errors::consensus::ConsensusError;
use crate::errors::ProtocolError;
use crate::identity::identity_public_key::KeyID;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Duplicated public keys ids {duplicated_ids:?} found")]
#[platform_serialize(unversioned)]
#[ferment_macro::export]
pub struct DuplicatedIdentityPublicKeyIdStateError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    pub duplicated_ids: Vec<KeyID>,
}

impl DuplicatedIdentityPublicKeyIdStateError {
    pub fn new(duplicated_ids: Vec<KeyID>) -> Self {
        Self { duplicated_ids }
    }

    pub fn duplicated_ids(&self) -> &Vec<KeyID> {
        &self.duplicated_ids
    }
}
impl From<DuplicatedIdentityPublicKeyIdStateError> for ConsensusError {
    fn from(err: DuplicatedIdentityPublicKeyIdStateError) -> Self {
        Self::StateError(StateError::DuplicatedIdentityPublicKeyIdStateError(err))
    }
}
