use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[error("$type is not present")]
pub struct MissingDocumentTransitionTypeError;

/*

DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

*/

impl MissingDocumentTransitionTypeError {
    pub fn new() -> Self {
        Self::default()
    }
}

impl From<MissingDocumentTransitionTypeError> for ConsensusError {
    fn from(err: MissingDocumentTransitionTypeError) -> Self {
        Self::BasicError(BasicError::MissingDocumentTransitionTypeError(err))
    }
}
