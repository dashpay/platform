use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Identity ${identity_id:?} already exists")]
pub struct IdentityAlreadyExistsError {
    identity_id: [u8; 32], // TODO Identifier?
}

impl IdentityAlreadyExistsError {
    pub fn new(identity_id: [u8; 32]) -> Self {
        Self { identity_id }
    }

    pub fn identity_id(&self) -> &[u8; 32] {
        &self.identity_id
    }
}

impl From<IdentityAlreadyExistsError> for ConsensusError {
    fn from(err: IdentityAlreadyExistsError) -> Self {
        Self::StateError(StateError::IdentityAlreadyExistsError(err))
    }
}
