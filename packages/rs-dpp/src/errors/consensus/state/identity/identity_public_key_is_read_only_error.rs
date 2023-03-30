use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::identity::KeyID;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[error("Identity Public Key #{public_key_index} is read only")]
pub struct IdentityPublicKeyIsReadOnlyError {
    public_key_index: KeyID,
}

impl IdentityPublicKeyIsReadOnlyError {
    pub fn new(public_key_index: KeyID) -> Self {
        Self { public_key_index }
    }

    pub fn public_key_index(&self) -> KeyID {
        self.public_key_index
    }
}
impl From<IdentityPublicKeyIsReadOnlyError> for ConsensusError {
    fn from(err: IdentityPublicKeyIsReadOnlyError) -> Self {
        Self::StateError(StateError::IdentityPublicKeyIsReadOnlyError(err))
    }
}
