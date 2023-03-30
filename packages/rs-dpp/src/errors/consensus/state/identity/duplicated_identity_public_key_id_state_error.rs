use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::identity::KeyID;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[error("Duplicated public keys ids {duplicated_ids:?} found")]
pub struct DuplicatedIdentityPublicKeyIdStateError {
    duplicated_ids: Vec<KeyID>,
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
