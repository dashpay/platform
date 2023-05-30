use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::identity::KeyID;
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
#[error("Duplicated public key ids ${duplicated_ids:?} found")]
pub struct DuplicatedIdentityPublicKeyIdBasicError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    duplicated_ids: Vec<KeyID>,
}

impl DuplicatedIdentityPublicKeyIdBasicError {
    pub fn new(duplicated_ids: Vec<KeyID>) -> Self {
        Self { duplicated_ids }
    }

    pub fn duplicated_ids(&self) -> &Vec<KeyID> {
        &self.duplicated_ids
    }
}
impl From<DuplicatedIdentityPublicKeyIdBasicError> for ConsensusError {
    fn from(err: DuplicatedIdentityPublicKeyIdBasicError) -> Self {
        Self::BasicError(BasicError::DuplicatedIdentityPublicKeyIdBasicError(err))
    }
}
