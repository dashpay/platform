use thiserror::Error;
use crate::errors::consensus::AbstractConsensusError;

#[derive(Error, Debug, Clone)]
#[error("Duplicated public key ids ${duplicated_ids} found")]
pub struct DuplicatedIdentityPublicKeyIdError {
    duplicated_ids: u64,
}

impl DuplicatedIdentityPublicKeyIdError {
    pub fn new(duplicated_ids: u64) -> Self {
        Self {
            duplicated_ids,
        }
    }

    pub fn duplicated_ids(&self) -> u64 {
        self.duplicated_ids
    }
}
