use thiserror::Error;

use crate::consensus::signature::SignatureError;
use crate::consensus::ConsensusError;
use crate::prelude::Identifier;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Identity {identity_id} not found")]
pub struct IdentityNotFoundError {
    identity_id: Identifier,
}

impl IdentityNotFoundError {
    pub fn new(identity_id: Identifier) -> Self {
        Self { identity_id }
    }

    pub fn identity_id(&self) -> Identifier {
        self.identity_id
    }
}

impl From<IdentityNotFoundError> for ConsensusError {
    fn from(err: IdentityNotFoundError) -> Self {
        Self::SignatureError(SignatureError::IdentityNotFoundError(err))
    }
}
