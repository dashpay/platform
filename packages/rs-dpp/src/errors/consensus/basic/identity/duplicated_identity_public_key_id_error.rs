use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Duplicated public key ids ${duplicated_ids:?} found")]
pub struct DuplicatedIdentityPublicKeyIdError {
    duplicated_ids: Vec<u64>,
}

impl DuplicatedIdentityPublicKeyIdError {
    pub fn new(duplicated_ids: Vec<u64>) -> Self {
        Self { duplicated_ids }
    }

    pub fn duplicated_ids(&self) -> &Vec<u64> {
        &self.duplicated_ids
    }
}
