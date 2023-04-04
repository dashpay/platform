use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[error("Identity cannot contain more than {max_items} public keys")]
pub struct MaxIdentityPublicKeyLimitReachedError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    max_items: usize,
}

impl MaxIdentityPublicKeyLimitReachedError {
    pub fn new(max_items: usize) -> Self {
        Self { max_items }
    }

    pub fn max_items(&self) -> usize {
        self.max_items
    }
}
impl From<MaxIdentityPublicKeyLimitReachedError> for ConsensusError {
    fn from(err: MaxIdentityPublicKeyLimitReachedError) -> Self {
        Self::StateError(StateError::MaxIdentityPublicKeyLimitReachedError(err))
    }
}
