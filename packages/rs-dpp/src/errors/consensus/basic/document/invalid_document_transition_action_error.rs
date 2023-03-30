use crate::consensus::basic::BasicError;
use thiserror::Error;
use crate::consensus::ConsensusError;

// TODO wrong param - in js action is number
#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Document transition action {} is not supported", action)]
pub struct InvalidDocumentTransitionActionError {
    action: String,
}

impl InvalidDocumentTransitionActionError {
    pub fn new(action: String) -> Self {
        Self { action }
    }

    pub fn action(&self) -> String {
        self.action.clone()
    }
}

impl From<InvalidDocumentTransitionActionError> for ConsensusError {
    fn from(err: InvalidDocumentTransitionActionError) -> Self {
        Self::BasicError(BasicError::InvalidDocumentTransitionActionError(err))
    }
}
