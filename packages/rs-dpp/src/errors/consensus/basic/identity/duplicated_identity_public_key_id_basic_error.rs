use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::identity::KeyID;
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Duplicated public key ids ${duplicated_ids:?} found")]
pub struct DuplicatedIdentityPublicKeyIdBasicError {
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
