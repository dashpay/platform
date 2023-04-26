use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::identity::KeyID;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[error("Identity Public Key with Ids {} do not exist", ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(", "))]
pub struct MissingIdentityPublicKeyIdsError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    ids: Vec<KeyID>,
}

impl MissingIdentityPublicKeyIdsError {
    pub fn new(ids: Vec<KeyID>) -> Self {
        Self { ids }
    }

    pub fn ids(&self) -> &Vec<KeyID> {
        &self.ids
    }
}
impl From<MissingIdentityPublicKeyIdsError> for ConsensusError {
    fn from(err: MissingIdentityPublicKeyIdsError) -> Self {
        Self::StateError(StateError::MissingIdentityPublicKeyIdsError(err))
    }
}
