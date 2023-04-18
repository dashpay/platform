use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Default, Eq, Serialize, Deserialize)]
#[error("$dataContractId is not present")]
pub struct MissingDataContractIdBasicError;

/*

DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

*/

impl MissingDataContractIdBasicError {
    pub fn new() -> Self {
        Self::default()
    }
}

impl From<MissingDataContractIdBasicError> for ConsensusError {
    fn from(err: MissingDataContractIdBasicError) -> Self {
        Self::BasicError(BasicError::MissingDataContractIdBasicError(err))
    }
}
