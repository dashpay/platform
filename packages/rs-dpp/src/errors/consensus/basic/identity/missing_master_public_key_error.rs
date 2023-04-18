use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[error(
    "Identity doesn't contain any master key, thus can not be updated. Please add a master key"
)]
pub struct MissingMasterPublicKeyError;

/*

DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

*/

impl MissingMasterPublicKeyError {
    pub fn new() -> Self {
        Self::default()
    }
}
impl From<MissingMasterPublicKeyError> for ConsensusError {
    fn from(err: MissingMasterPublicKeyError) -> Self {
        Self::BasicError(BasicError::MissingMasterPublicKeyError(err))
    }
}
