use crate::identity::KeyID;
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Duplicated public keys ${duplicated_ids:?} found")]
pub struct DuplicatedIdentityPublicKeyError {
    duplicated_ids: Vec<KeyID>,
}

impl DuplicatedIdentityPublicKeyError {
    pub fn new(duplicated_ids: Vec<KeyID>) -> Self {
        Self { duplicated_ids }
    }

    pub fn duplicated_public_keys_ids(&self) -> &Vec<KeyID> {
        &self.duplicated_ids
    }
}
