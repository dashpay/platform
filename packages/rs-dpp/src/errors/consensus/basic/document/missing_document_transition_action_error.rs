use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[error("$action is not present")]
pub struct MissingDocumentTransitionActionError;

impl MissingDocumentTransitionActionError {
    pub fn new() -> Self {
        Self::default()
    }
}

impl From<MissingDocumentTransitionActionError> for ConsensusError {
    fn from(err: MissingDocumentTransitionActionError) -> Self {
        Self::BasicError(BasicError::MissingDocumentTransitionActionError(err))
    }
}
