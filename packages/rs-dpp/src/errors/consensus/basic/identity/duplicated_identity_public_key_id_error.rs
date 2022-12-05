use crate::identity::KeyID;
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Duplicated public key ids ${duplicated_ids:?} found")]
pub struct DuplicatedIdentityPublicKeyIdError {
    duplicated_ids: Vec<KeyID>,
}

impl DuplicatedIdentityPublicKeyIdError {
    pub fn new(duplicated_ids: Vec<KeyID>) -> Self {
        Self { duplicated_ids }
    }

    pub fn duplicated_ids(&self) -> &Vec<KeyID> {
        &self.duplicated_ids
    }
}
