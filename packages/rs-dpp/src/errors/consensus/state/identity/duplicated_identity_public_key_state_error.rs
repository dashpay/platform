use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use thiserror::Error;
use crate::identity::KeyID;

use crate::prelude::{Revision};

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Duplicated public keys {duplicated_public_key_ids:?} found")]
pub struct DuplicatedIdentityPublicKeyStateError {
    duplicated_public_key_ids: Vec<KeyID>,
}

impl DuplicatedIdentityPublicKeyStateError {
    pub fn new( duplicated_public_key_ids: Vec<KeyID>) -> Self {
        Self {
            duplicated_public_key_ids
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
