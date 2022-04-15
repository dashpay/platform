use thiserror::Error;

#[derive(Error, Debug, Clone)]
#[error("Duplicated public keys ${duplicated_ids:?} found")]
pub struct DuplicatedIdentityPublicKeyError {
    duplicated_ids: Vec<u64>,
}

impl DuplicatedIdentityPublicKeyError {
    pub fn new(duplicated_ids: Vec<u64>) -> Self {
        Self {
            duplicated_ids,
        }
    }

    pub fn duplicated_public_keys_ids(&self) -> &Vec<u64> {
        &self.duplicated_ids
    }
}
