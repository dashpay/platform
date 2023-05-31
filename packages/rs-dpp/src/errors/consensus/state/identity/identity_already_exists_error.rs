use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use bincode::{Decode, Encode};
use platform_value::Identifier;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
#[error("Identity ${identity_id:?} already exists")]
pub struct IdentityAlreadyExistsError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    identity_id: Identifier,
}

impl IdentityAlreadyExistsError {
    pub fn new(identity_id: Identifier) -> Self {
        Self { identity_id }
    }

    pub fn identity_id(&self) -> Identifier {
        self.identity_id
    }
}

impl From<IdentityAlreadyExistsError> for ConsensusError {
    fn from(err: IdentityAlreadyExistsError) -> Self {
        Self::StateError(StateError::IdentityAlreadyExistsError(err))
    }
}
